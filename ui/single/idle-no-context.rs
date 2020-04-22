#![no_main]

#[mock::app]
mod APP {
    #[idle]
    fn idle() -> ! {
        loop {}
    }
}
