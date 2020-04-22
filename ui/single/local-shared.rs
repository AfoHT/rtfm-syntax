#![no_main]

#[mock::app]
mod APP {
    #[idle]
    fn idle(_: idle::Context) -> ! {
        #[shared]
        static mut X: [u8; 128] = [0; 128];

        loop {}
    }
}
