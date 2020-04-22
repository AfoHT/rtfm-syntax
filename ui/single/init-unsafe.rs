#![no_main]

#[mock::app]
mod APP {
    #[init]
    unsafe fn init(_: init::Context) {}
}
