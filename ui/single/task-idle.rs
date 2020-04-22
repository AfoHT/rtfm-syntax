#![no_main]

#[mock::app]
mod APP {
    #[idle]
    fn foo(_: foo::Context) -> ! {
        loop {}
    }

    // name collides with `#[idle]` function
    #[task]
    fn foo(_: foo::Context) {}
}
