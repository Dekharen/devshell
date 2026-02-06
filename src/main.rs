mod cli;
mod config;
mod fragments;
mod docker;
mod fs;
mod error;
mod util;

fn main() {
    cli::run();
}
