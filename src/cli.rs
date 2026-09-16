use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum LinkMode {
    #[default]
    Inline,
    Text,
    Hyperlink,
    Reference,
}
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum ThemeMode {
    #[default]
    Dark,
    Light,
}

#[derive(Parser, Debug)]
#[command(name = "rustdown", version, about = "Render Markdown to the terminal")]
pub(crate) struct Cli {
    /// A file path or inline Markdown. Reads stdin if omitted.
    pub(crate) input: Option<String>,
    /// Preserve original fenced-code delimiters and info strings.
    #[arg(long, global = true)]
    pub(crate) preserve_fences: bool,
    /// Wrap code to the available width; expand tabs at four-column stops.
    #[arg(long, global = true)]
    pub(crate) wrap_code: bool,
    /// Layout width in columns. Defaults to the terminal width, or 80 when redirected.
    #[arg(long, global = true, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub(crate) width: Option<u16>,
    /// Color policy. Auto honors NO_COLOR and disables colors when redirected.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) color: ColorMode,
    /// Use ASCII borders, bullets, and checkboxes.
    #[arg(long, global = true)]
    pub(crate) ascii: bool,
    /// Display inline URLs, labels, clickable hyperlinks, or numbered references.
    #[arg(long, global = true, value_enum, default_value = "inline")]
    pub(crate) links: LinkMode,
    /// Enable syntax highlighting for recognized fenced-code languages.
    #[arg(long, global = true)]
    pub(crate) highlight: bool,
    /// Color palette for the terminal background, including syntax highlighting.
    #[arg(long, global = true, value_enum, default_value = "dark")]
    pub(crate) theme: ThemeMode,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Convert a whole HTML document to Markdown and render it.
    Html { input: Option<String> },
}
