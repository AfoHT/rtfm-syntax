#![no_main]

#[mock::app(cores = 2, parse_cores)]
mod APP {
    struct Resources {
        a: u32,
    }

    #[init(core = 0)]
    fn init(_: init::Context) -> init::LateResources {}

    #[init(core = 1)]
    fn init(_: init::Context) -> init::LateResources {}
}
