//! RTIC application analysis

use core::cmp;
use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexMap;
use syn::{Ident, Type};

use crate::{ast::App, Id, Set};

pub(crate) fn app(app: &App) -> Analysis {
    // a. Which id initializes which resources
    let mut late_resources = LateResources::new();
    if !app.late_resources.is_empty() {
        let mut resources = app.late_resources.keys().cloned().collect::<BTreeSet<_>>();
        let mut rest = None;
        if let Some(init) = &app.inits.first() {
            if init.args.late.is_empty() {
                // this was checked in the `check` pass
                debug_assert!(rest.is_none());

                rest = Some(());
            } else {
                //let late_resources = late_resources.entry(0).or_default();
                let mut late_resources = Vec::new();

                for name in &init.args.late {
                    late_resources.push(name.clone());
                    resources.remove(name);
                }
            }
        }

        if let Some(_rest) = rest {
            late_resources.push(resources);
        }
    }

    // c. Ceiling analysis of Exclusive resources
    // d. Sync-ness of Access::Shared resources
    // e. Location of resources
    let mut locations = app
        .late_resources
        .iter()
        .chain(app.resources.iter().map(|(name, res)| (name, &res.late)))
        .filter_map(|(_name, _lr)| {
                None
        })
        .collect::<Locations>();

    let mut ownerships = Ownerships::new();
    let mut sync_types = SyncTypes::new();
    for (prio, name, access) in app.resource_accesses() {
        let res = app.resource(name).expect("UNREACHABLE").0;

        // (e)
        // Add each resource to locations
        locations.insert(
            name.clone(),
            Location::Owned,
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
                            sync_types.insert(res.ty.clone());
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
        .iter()
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
    let mut tq = TimerQueue::default();
    for (_, name) in app.schedule_calls() {
        let schedulee = &app.software_tasks[name];
        let schedulee_prio = schedulee.args.priority;

        tq.tasks.insert(name.clone());

        // the handler priority must match the priority of the highest priority schedulee
        tq.priority = cmp::max(tq.priority, schedulee_prio);

        // the priority ceiling must be equal or greater than the handler priority
        tq.ceiling = tq.priority;
    }

    // g. Ceiling analysis of free queues (consumer end point) -- first pass
    // h. Ceiling analysis of the channels (producer end point) -- first pass (#TODO MULTICORE)
    // j. Send analysis
    let mut free_queues = FreeQueues::new();
    for (spawner_prio, name) in app.spawn_calls() {
        let spawnee = &app.software_tasks[name];
        let spawnee_prio = spawnee.args.priority;

        let mut must_be_send = false;

        let fq = free_queues
            .entry(name.clone())
            .or_default()
            //#TODO
            .entry(0)
            .or_default();

        if let Some(prio) = spawner_prio {
            // (g) Spawners contend for the free queue
            match *fq {
                None => *fq = Some(prio),
                Some(ceil) => *fq = Some(cmp::max(ceil, prio)),
            }

            // (j) messages that connect tasks running at different priorities need to be
            // `Send`
            if spawnee_prio != prio {
                must_be_send = true;
            }
        } else {
            // (g, h) spawns from `init` are excluded from the ceiling analysis
            // (j) but spawns from `init` must be `Send`
            must_be_send = true;
        }

        if must_be_send {
            {
                //#TODO
                let send_types = send_types.entry(0).or_default();

                spawnee.inputs.iter().for_each(|input| {
                    send_types.insert(input.ty.clone());
                });
            }

            //#TODO
            let send_types = send_types.entry(0).or_default();

            spawnee.inputs.iter().for_each(|input| {
                send_types.insert(input.ty.clone());
            });
        }
    }

    // k. Ceiling analysis of free queues (consumer end point) -- second pass
    // l. Ceiling analysis of the channels (producer end point) -- second pass (#TODO MULTICORE)
    // m. Ceiling analysis of the timer queue
    // n. Spawn barriers analysis (schedule edition)
    // o. Send analysis

    for (scheduler_prio, name) in app.schedule_calls() {
        let schedulee = &app.software_tasks[name];
        let schedulee_prio = schedulee.args.priority;

        let mut must_be_send = false;


        let fq = free_queues
            .entry(name.clone())
            .or_default()
            .entry(0)
            .or_default();

        if let Some(prio) = scheduler_prio {
            // (k) Schedulers contend for the free queue
            match *fq {
                None => *fq = Some(prio),
                Some(ceil) => *fq = Some(cmp::max(ceil, prio)),
            }

            // (m) Schedulers contend for the timer queue
            tq.ceiling = cmp::max(tq.ceiling, prio);

            // (o) messages that connect tasks running at different priorities need to be
            // `Send`
            if schedulee_prio != prio {
                must_be_send = true;
            }
        } else {
            // (k, m) schedules from `init` are excluded from the ceiling analysis
            // (o) but schedules from `init` must be `Send`
            must_be_send = true;
        }

        if must_be_send {
            {
                let send_types = send_types.entry(0).or_default();

                schedulee.inputs.iter().for_each(|input| {
                    send_types.insert(input.ty.clone());
                });
            }

            //#TODO
            let send_types = send_types.entry(0).or_default();

            schedulee.inputs.iter().for_each(|input| {
                send_types.insert(input.ty.clone());
            });
        }
    }

    // Compute the capacity of the timer queues
    tq.capacity = tq
        .tasks
        .iter()
        .map(|name| app.software_tasks[name].args.capacity)
        .sum();

    Analysis {
        free_queues,
        late_resources,
        locations,
        ownerships,
        send_types,
        sync_types,
        timer_queue: tq,
    }
}

/// Priority ceiling
pub type Ceiling = Option<u8>;

/// Task priority
pub type Priority = u8;

/// Resource name
pub type Resource = Ident;

/// Task name
pub type Task = Ident;

/// The result of analyzing an RTIC application
pub struct Analysis {
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

    /// Timer queue
    pub timer_queue: TimerQueue,
}

/// All free queues, keyed by task and then by Id
pub type FreeQueues = IndexMap<Task, BTreeMap<Id, Ceiling>>;

/// Late resources, wrapped in a vector
pub type LateResources = Vec<BTreeSet<Resource>>;

/// Location of all *used* resources
pub type Locations = IndexMap<Resource, Location>;

/// Resource ownership
pub type Ownerships = IndexMap<Resource, Ownership>;

/// These types must implement the `Send` trait
pub type SendTypes = BTreeMap<Id, Set<Box<Type>>>;

/// These types must implement the `Sync` trait
//pub type SyncTypes = BTreeMap<Id, Set<Box<Type>>>;
pub type SyncTypes = Set<Box<Type>>;

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
    /// resource that resides in `id`
    Owned
}

/*
impl Location {
    /// If resource is owned this returns the Id owning it
    pub fn id(&self) -> Option<u8> {
        match *self {
            Location::Owned { id, .. } => Some(id),
        }
    }
}
*/
