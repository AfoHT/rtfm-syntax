#![no_main]

#[mock::app]
mod APP {
    #[idle(resources = [A], resources = [B])]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
