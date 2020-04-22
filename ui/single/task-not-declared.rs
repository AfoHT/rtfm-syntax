#![no_main]

#[mock::app]
mod APP {
    #[init(spawn = [foo])]
    fn init(_: init::Context) {}
}
