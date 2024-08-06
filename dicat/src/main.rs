use clap::Parser;
use dicat::{prompt_parser::Args, App};
fn main() {
    let args = Args::parse();

    if let Err(err) = App::start(args) {
        eprintln!("Error: {}.", err);
        std::process::exit(1);
    }
}
