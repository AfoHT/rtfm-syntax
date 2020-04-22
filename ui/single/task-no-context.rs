#![no_main]

#[mock::app]
mod APP {
    #[task]
    fn foo() {}
}
