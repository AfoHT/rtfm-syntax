#![no_main]

#[mock::app]
mod APP {
    struct Resources {
        x: u32,
    }

    #[init(resources = [x])]
    fn init(_: init::Context) {}
}
