#![no_main]

#[mock::app]
mod APP {
    #[idle(spawn = [foo], spawn = [bar])]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
