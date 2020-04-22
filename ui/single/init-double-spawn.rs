#![no_main]

#[mock::app]
mod APP {
    #[init(spawn = [foo], spawn = [bar])]
    fn init(_: init::Context) {}
}
