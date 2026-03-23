use clap::{Parser, Subcommand};

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

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Convert HTML to Markdown and render it
    Html {
        /// HTML string to convert and render
        input: Option<String>,

        /// Preserve original fenced code fences
        #[arg(long = "preserve-fences")]
        preserve_fences: bool,
    },
}
