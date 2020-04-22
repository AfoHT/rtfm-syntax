#![no_main]

#[mock::app(cores = 2, parse_cores)]
mod APP {
    #[init]
    fn init(_: init::Context) {}
}
