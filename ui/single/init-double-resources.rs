#![no_main]

#[mock::app]
mod APP {
    #[init(resources = [A], resources = [B])]
    fn init(_: init::Context) {}
}
