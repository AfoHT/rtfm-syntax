#![no_main]

#[mock::app]
mod APP {
    #[resources]
    struct Resources {
        #[init(0)]
        pub x: u32,
    }
}
