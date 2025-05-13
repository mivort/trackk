use clap::Parser;

mod args;
mod issue;

fn main() {
    let _args = args::Args::parse();
}
