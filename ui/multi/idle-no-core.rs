#![no_main]

#[mock::app(cores = 2, parse_cores)]
mod APP {
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
