#![no_main]

#[mock::app]
mod APP {
    #[task]
    pub fn foo(_: foo::Context) {}
}
