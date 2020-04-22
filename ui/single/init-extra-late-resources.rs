#![no_main]

#[mock::app]
mod APP {
    #[init]
    fn init(_: init::Context) -> init::LateResources {}
}
