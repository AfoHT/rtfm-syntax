#![no_main]

#[mock::app(cores = 2, parse_cores, parse_extern_interrupt)]
mod APP {
    extern "C" {
        fn foo();
    }
}
