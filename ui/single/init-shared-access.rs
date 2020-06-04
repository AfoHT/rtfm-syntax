#![no_main]

#[mock::app]
mod APP {
    #[resources]
    struct Resources {
        #[init(0)]
        x: i32,
    }

    #[init(resources = [&x])]
    fn init(_: init::Context) {}
}
