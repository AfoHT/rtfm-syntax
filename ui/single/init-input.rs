#![no_main]

#[mock::app]
mod APP {
    #[init]
    fn init(_: init::Context, _undef: u32) {}
}
