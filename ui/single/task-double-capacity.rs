#![no_main]

#[mock::app]
mod APP {
    #[task(capacity = 1, capacity = 2)]
    fn foo(_: foo::Context) {}
}
