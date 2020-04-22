#![no_main]

#[mock::app]
mod APP {
    #[task(resources = [A])]
    fn foo(_: foo::Context) {}
}
