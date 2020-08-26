//! RTIC application analysis

use core::cmp;
use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexMap;
use syn::{Ident, Type};

use crate::{ast::App, Core, Set};

pub(crate) fn app(app: &App) -> Analysis {
    // a. Which core initializes which resources
    let mut late_resources = LateResources::new();
    if !app.late_resources.is_empty() {
        let mut resources = app.late_resources.keys().cloned().collect::<BTreeSet<_>>();
        let mut rest = None;
        for (&core, init) in &app.inits {
            if init.args.late.is_empty() {
                // this was checked in the `check` pass
                debug_assert!(rest.is_none());

                rest = Some(core);
            } else {
                let late_resources = late_resources.entry(core).or_default();

                for name in &init.args.late {
                    late_resources.insert(name.clone());
                    resources.remove(name);
                }
            }
        }

        if let Some(rest) = rest {
            late_resources.insert(rest, resources);
        }
    }

    // c. Ceiling analysis of Exclusive resources
    // d. Sync-ness of Access::Shared resources
    // e. Location of resources
    // f. Cross initialization needs a post-initialization synchronization barrier
    let mut initialization_barriers = InitializationBarriers::new();
    let mut locations = app
        .late_resources
        .iter()
        .chain(app.resources.iter().map(|(name, res)| (name, &res.late)))
        .filter_map(|(_name, _lr)| {
                None
        })
        .collect::<Locations>();

    let mut ownerships = Ownerships::new();
    //let mut shared_accesses = HashMap::new();
    let mut sync_types = SyncTypes::new();
    for (core, prio, name, access) in app.resource_accesses() {
        let res = app.resource(name).expect("UNREACHABLE").0;

        // (e)
        // Add each resource to locations
        locations.insert(
            name.clone(),
            Location::Owned {
                core,
            },
        );

        // (c)
        if let Some(priority) = prio {
            if let Some(ownership) = ownerships.get_mut(name) {
                match *ownership {
                    Ownership::Owned { priority: ceiling }
                    | Ownership::CoOwned { priority: ceiling }
                    | Ownership::Contended { ceiling }
                        if priority != ceiling =>
                    {
                        *ownership = Ownership::Contended {
                            ceiling: cmp::max(ceiling, priority),
                        };

                        if access.is_shared() {
                            sync_types.entry(core).or_default().insert(res.ty.clone());
                        }
                    }

                    Ownership::Owned { priority: ceil } if ceil == priority => {
                        *ownership = Ownership::CoOwned { priority };
                    }

                    _ => {}
                }
            } else {
                ownerships.insert(name.clone(), Ownership::Owned { priority });
            }
        }

        // (f) in cross-initialization the initializer core is like a sender and the user core is
        // like a receiver
        let receiver = core;
        for (&sender, resources) in &late_resources {
            if sender == receiver {
                continue;
            }

            if resources.contains(name) {
                initialization_barriers
                    .entry(receiver)
                    .or_default()
                    .insert(sender);
            }
        }
    }

    // Most late resources need to be `Send`
    let mut send_types = SendTypes::new();
    let owned_by_idle = Ownership::Owned { priority: 0 };
    for (name, res) in app.late_resources.iter() {
        // handle not owned by idle
        if ownerships
            .get(name)
            .map(|ownership| *ownership != owned_by_idle)
            .unwrap_or(false)
            {
                send_types.entry(0).or_default().insert(res.ty.clone());
            }
    }

    // All resources shared with `init` (ownership != None) need to be `Send`
    for name in app
        .inits
        .values()
        .flat_map(|init| init.args.resources.keys())
    {
        if let Some(ownership) = ownerships.get(name) {
            if *ownership != owned_by_idle {
                send_types
                    .entry(0)
                    .or_default()
                    .insert(app.resources[name].ty.clone());
            }
        }
    }

    // Initialize the timer queues
    let mut timer_queues = TimerQueues::new();
    for (scheduler_core, _scheduler_prio, name) in app.schedule_calls() {
        let schedulee = &app.software_tasks[name];
        let schedulee_prio = schedulee.args.priority;

        let tq = timer_queues.entry(scheduler_core).or_default();
        tq.tasks.insert(name.clone());

        // the handler priority must match the priority of the highest priority schedulee that's
        // dispatched in the same core
        tq.priority = cmp::max(tq.priority, schedulee_prio);

        // the priority ceiling must be equal or greater than the handler priority
        tq.ceiling = tq.priority;
    }

    // g. Ceiling analysis of free queues (consumer end point) -- first pass
    // h. Ceiling analysis of the channels (producer end point) -- first pass
    // i. Spawn barriers analysis
    // j. Send analysis
    let mut channels = Channels::new();
    let mut free_queues = FreeQueues::new();
    for (spawner_core, spawner_prio, name) in app.spawn_calls() {
        let spawnee = &app.software_tasks[name];
        let spawnee_core = spawnee.args.core;
        let spawnee_prio = spawnee.args.priority;

        let mut must_be_send = false;

        let channel = channels
            .entry(spawnee_core)
            .or_default()
            .entry(spawnee_prio)
            .or_default()
            .entry(spawner_core)
            .or_default();
        channel.tasks.insert(name.clone());

        let fq = free_queues
            .entry(name.clone())
            .or_default()
            .entry(spawner_core)
            .or_default();

        if let Some(prio) = spawner_prio {
            // (h) Spawners contend for the `channel`
            match channel.ceiling {
                None => channel.ceiling = Some(prio),
                Some(ceil) => channel.ceiling = Some(cmp::max(prio, ceil)),
            }

            // (g) Spawners contend for the free queue
            match *fq {
                None => *fq = Some(prio),
                Some(ceil) => *fq = Some(cmp::max(ceil, prio)),
            }

            // (j) core-local messages that connect tasks running at different priorities need to be
            // `Send`
            if spawner_core == spawnee_core && spawnee_prio != prio {
                must_be_send = true;
            }
        } else if spawner_core == spawnee_core {
            // (g, h) spawns from `init` are excluded from the ceiling analysis
            // (j) but spawns from `init` must be `Send`
            must_be_send = true;
        }

        if must_be_send {
            {
                let send_types = send_types.entry(spawner_core).or_default();

                spawnee.inputs.iter().for_each(|input| {
                    send_types.insert(input.ty.clone());
                });
            }

            let send_types = send_types.entry(spawnee_core).or_default();

            spawnee.inputs.iter().for_each(|input| {
                send_types.insert(input.ty.clone());
            });
        }
    }

    // k. Ceiling analysis of free queues (consumer end point) -- second pass
    // l. Ceiling analysis of the channels (producer end point) -- second pass
    // m. Ceiling analysis of the timer queue
    // n. Spawn barriers analysis (schedule edition)
    // o. Send analysis
    for (scheduler_core, scheduler_prio, name) in app.schedule_calls() {
        let schedulee = &app.software_tasks[name];
        let schedulee_core = schedulee.args.core;
        let schedulee_prio = schedulee.args.priority;

        let mut must_be_send = false;

        let tq = timer_queues.get_mut(&scheduler_core).expect("UNREACHABLE");

        let channel = channels
            .entry(schedulee_core)
            .or_default()
            .entry(schedulee_prio)
            .or_default()
            .entry(scheduler_core)
            .or_default();
        channel.tasks.insert(name.clone());

        let fq = free_queues
            .entry(name.clone())
            .or_default()
            .entry(scheduler_core)
            .or_default();

        // (l) The timer queue handler contends for the `channel`
        match channel.ceiling {
            None => channel.ceiling = Some(tq.priority),
            Some(ceil) => channel.ceiling = Some(cmp::max(ceil, tq.priority)),
        }

        if let Some(prio) = scheduler_prio {
            // (k) Schedulers contend for the free queue
            match *fq {
                None => *fq = Some(prio),
                Some(ceil) => *fq = Some(cmp::max(ceil, prio)),
            }

            // (m) Schedulers contend for the timer queue
            tq.ceiling = cmp::max(tq.ceiling, prio);

            // (o) core-local messages that connect tasks running at different priorities need to be
            // `Send`
            if scheduler_core == schedulee_core && schedulee_prio != prio {
                must_be_send = true;
            }
        } else if scheduler_core == schedulee_core {
            // (k, m) schedules from `init` are excluded from the ceiling analysis
            // (o) but schedules from `init` must be `Send`
            must_be_send = true;
        }

        if must_be_send {
            {
                let send_types = send_types.entry(scheduler_core).or_default();

                schedulee.inputs.iter().for_each(|input| {
                    send_types.insert(input.ty.clone());
                });
            }

            let send_types = send_types.entry(schedulee_core).or_default();

            schedulee.inputs.iter().for_each(|input| {
                send_types.insert(input.ty.clone());
            });
        }
    }

    // no channel should ever be empty
    debug_assert!(channels.values().all(|dispatchers| dispatchers
        .values()
        .all(|channels| channels.values().all(|channel| !channel.tasks.is_empty()))));

    // Compute channel capacities
    for channel in channels
        .values_mut()
        .flat_map(|dispatchers| dispatchers.values_mut())
        .flat_map(|dispatcher| dispatcher.values_mut())
    {
        channel.capacity = channel
            .tasks
            .iter()
            .map(|name| app.software_tasks[name].args.capacity)
            .sum();
    }

    // Compute the capacity of the timer queues
    for tq in timer_queues.values_mut() {
        tq.capacity = tq
            .tasks
            .iter()
            .map(|name| app.software_tasks[name].args.capacity)
            .sum();
    }

    let used_cores = app
        .inits
        .keys()
        .cloned()
        .chain(app.idles.keys().cloned())
        .chain(app.hardware_tasks.values().map(|task| task.args.core))
        .chain(app.software_tasks.values().map(|task| task.args.core))
        .collect();

    Analysis {
        used_cores,
        channels,
        free_queues,
        initialization_barriers,
        late_resources,
        locations,
        ownerships,
        send_types,
        sync_types,
        timer_queues,
    }
}

/// Priority ceiling
pub type Ceiling = Option<u8>;

/// Task priority
pub type Priority = u8;

/// Receiver core
pub type Receiver = Core;

/// Resource name
pub type Resource = Ident;

/// Sender core
pub type Sender = Core;

/// Task name
pub type Task = Ident;

/// The result of analyzing an RTIC application
pub struct Analysis {
    /// Cores that have been assigned at least task, `#[init]` or `#[idle]`
    pub used_cores: BTreeSet<Core>,

    /// SPSC message channels between cores
    pub channels: Channels,

    /// Priority ceilings of "free queues"
    pub free_queues: FreeQueues,

    /// Maps a core to the late resources it initializes
    pub late_resources: LateResources,

    /// Location of all *used* resources
    ///
    /// If a resource is not listed here it means that's a "dead" (never accessed) resource and the
    /// backend should not generate code for it
    ///
    /// `None` indicates that the resource must reside in memory visible to more than one core
    /// ("shared memory")
    pub locations: Locations,

    /// Resource ownership
    pub ownerships: Ownerships,

    /// These types must implement the `Send` trait
    pub send_types: SendTypes,

    /// These types must implement the `Sync` trait
    pub sync_types: SyncTypes,

    /// Cross-core initialization barriers
    pub initialization_barriers: InitializationBarriers,

    /// Timer queues
    pub timer_queues: TimerQueues,
}

/// All cross-core channels, keyed by receiver core, then by dispatch priority and then by sender
/// core
pub type Channels = BTreeMap<Receiver, BTreeMap<Priority, BTreeMap<Sender, Channel>>>;

/// All free queues, keyed by task and then by sender
pub type FreeQueues = IndexMap<Task, BTreeMap<Sender, Ceiling>>;

/// Late resources, keyed by the core that initializes them
pub type LateResources = BTreeMap<Core, BTreeSet<Resource>>;

/// Location of all *used* resources
pub type Locations = IndexMap<Resource, Location>;

/// Resource ownership
pub type Ownerships = IndexMap<Resource, Ownership>;

/// These types must implement the `Send` trait
pub type SendTypes = BTreeMap<Core, Set<Box<Type>>>;

/// These types must implement the `Sync` trait
pub type SyncTypes = BTreeMap<Core, Set<Box<Type>>>;

/// Cross-core initialization barriers
pub type InitializationBarriers =
    BTreeMap</* user */ Receiver, BTreeSet</* initializer */ Sender>>;

/// Cross-core spawn barriers
pub type SpawnBarriers =
    BTreeMap</* spawnee */ Receiver, BTreeMap</* spawner */ Sender, /* before_init */ bool>>;

/// Timer queues, keyed by core
pub type TimerQueues = BTreeMap<Core, TimerQueue>;

/// The timer queue
#[derive(Debug)]
pub struct TimerQueue {
    /// The capacity of the queue
    pub capacity: u8,

    /// The priority ceiling of the queue
    pub ceiling: u8,

    /// Priority of the timer queue handler
    pub priority: u8,

    /// Tasks that can be scheduled on this queue
    pub tasks: BTreeSet<Task>,
}

impl Default for TimerQueue {
    fn default() -> Self {
        Self {
            capacity: 0,
            ceiling: 1,
            priority: 1,
            tasks: BTreeSet::new(),
        }
    }
}

/// A channel between cores used to send messages
#[derive(Debug, Default)]
pub struct Channel {
    /// The channel capacity
    pub capacity: u8,

    /// The (sender side) priority ceiling of this SPSC channel
    pub ceiling: Ceiling,

    /// Tasks that can be spawned on this channel
    pub tasks: BTreeSet<Task>,
}

/// Resource ownership
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ownership {
    /// Owned by a single task
    Owned {
        /// Priority of the task that owns this resource
        priority: u8,
    },

    /// "Co-owned" by more than one task; all of them have the same priority
    CoOwned {
        /// Priority of the tasks that co-own this resource
        priority: u8,
    },

    /// Contended by more than one task; the tasks have different priorities
    Contended {
        /// Priority ceiling
        ceiling: u8,
    },
}

impl Ownership {
    /// Whether this resource needs to a lock at this priority level
    pub fn needs_lock(&self, priority: u8) -> bool {
        match self {
            Ownership::Owned { .. } | Ownership::CoOwned { .. } => false,

            Ownership::Contended { ceiling } => {
                debug_assert!(*ceiling >= priority);

                priority < *ceiling
            }
        }
    }

    /// Whether this resource is exclusively owned
    pub fn is_owned(&self) -> bool {
        match self {
            Ownership::Owned { .. } => true,
            _ => false,
        }
    }
}

/// Resource location
#[derive(Clone, Debug, PartialEq)]
pub enum Location {
    /// resource that resides in `core`
    Owned {
        /// Core on which this resource is located
        core: u8,
    },
}

impl Location {
    /// If resource is owned this returns the core on which is located
    pub fn core(&self) -> Option<u8> {
        match *self {
            Location::Owned { core, .. } => Some(core),
        }
    }
}
