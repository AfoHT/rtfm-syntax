#![no_main]

#[mock::app]
mod APP {
    #[task(priority = 0)]
    fn foo(_: foo::Context) {}
}
