#![no_main]

#[mock::app]
mod APP {
    #[resources]
    struct Resources {
        x: u32,
    }

    #[init(resources = [x])]
    fn init(_: init::Context) {}
}
