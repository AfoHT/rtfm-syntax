#![no_main]

#[mock::app]
mod APP {
    #[init]
    pub fn init(_: init::Context) {}
}
