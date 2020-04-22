#![no_main]

#[mock::app]
mod APP {
    #[task]
    unsafe fn foo(_: foo::Context) {}
}
