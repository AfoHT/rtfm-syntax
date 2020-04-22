#![no_main]

#[mock::app]
mod APP {
    #[task]
    fn foo(_: foo::Context) -> ! {
        loop {}
    }
}
