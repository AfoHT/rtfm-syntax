#![no_main]

#[mock::app]
mod APP {
    #[idle]
    unsafe fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
