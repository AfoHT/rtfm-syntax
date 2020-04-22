#![no_main]

#[mock::app]
mod APP {
    #[idle]
    fn idle(_: idle::Context) {}
}
