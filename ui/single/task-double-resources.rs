#![no_main]

#[mock::app]
mod APP {
    #[task(resources = [A], resources = [B])]
    fn foo(_: foo::Context) {}
}
