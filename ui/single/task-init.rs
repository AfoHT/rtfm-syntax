#![no_main]

#[mock::app]
mod APP {
    #[init]
    fn foo(_: foo::Context) {}

    // name collides with `#[idle]` function
    #[task]
    fn foo(_: foo::Context) {}
}
