#![no_main]

#[mock::app]
mod APP {
    struct Resources {
        x: u32,
    }

    #[init]
    fn init(_: init::Context) {}
}
