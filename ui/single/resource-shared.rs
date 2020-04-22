#![no_main]

#[mock::app]
mod APP {
    struct Resources {
        #[shared]
        x: u32,
    }
}
