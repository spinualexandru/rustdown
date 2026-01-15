use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rustdown",
    about = "Render Markdown to the terminal with colors"
)]
pub(crate) struct Cli {
    /// Input Markdown file.
    pub(crate) file: PathBuf,

    /// Preserve original fenced code fences (like ```lang ... ```)
    #[arg(long = "preserve-fences")]
    pub(crate) preserve_fences: bool,
}
