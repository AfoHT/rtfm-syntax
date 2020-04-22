#![no_main]

#[mock::app]
mod APP {
    struct Resources {
        #[init(0)]
        pub x: u32,
    }
}
