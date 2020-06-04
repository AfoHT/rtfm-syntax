#![no_main]

#[mock::app]
mod APP {
    #[resources]
    struct Resources {
        #[shared]
        x: u32,
    }
}
