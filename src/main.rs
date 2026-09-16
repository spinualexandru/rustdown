mod app;
mod cli;

use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    let cli = crate::cli::Cli::parse();
    crate::app::run(cli)
}
