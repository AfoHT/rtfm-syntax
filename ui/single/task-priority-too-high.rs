#![no_main]

#[mock::app]
mod APP {
    #[task(priority = 256)]
    fn foo(_: foo::Context) {}
}
