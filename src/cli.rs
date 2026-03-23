use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "rustdown",
    about = "Render Markdown to the terminal with colors"
)]
pub(crate) struct Cli {
    /// Input: a file path or inline Markdown string. Reads from stdin if omitted.
    pub(crate) input: Option<String>,

    /// Preserve original fenced code fences (like ```lang ... ```)
    #[arg(long = "preserve-fences")]
    pub(crate) preserve_fences: bool,
}
