use worker::start;

mod lib;

fn main() {
    start!(lib::main);
}
