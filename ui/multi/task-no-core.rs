#![no_main]

#[mock::app(cores = 2, parse_cores)]
mod APP {
    #[task]
    fn foo(_: foo::Context) {}
}
