#![no_main]

#[mock::app]
mod APP {
    #[idle]
    pub fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
