mod cli;
mod config;
mod docker;
mod error;
mod fragments;
mod fs;
mod util;

fn main() {
    cli::run();
}
