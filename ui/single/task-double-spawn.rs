#![no_main]

#[mock::app]
mod APP {
    #[task(spawn = [bar], spawn = [baz])]
    fn foo(_: foo::Context) {}
}
