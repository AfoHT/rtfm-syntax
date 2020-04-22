#![no_main]

#[mock::app(cores = 2, parse_cores)]
mod APP {
    struct Resources {
        a: u32,
        b: u32,
        c: u32,
    }

    #[init(core = 0, late = [a])]
    fn init(_: init::Context) -> init::LateResources {}

    #[init(core = 1, late = [b])]
    fn init(_: init::Context) -> init::LateResources {}
}
