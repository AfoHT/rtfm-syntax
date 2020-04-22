#![no_main]

#[mock::app]
mod APP {
    #[idle]
    fn idle(_: idle::Context, _undef: u32) -> ! {
        loop {}
    }
}
