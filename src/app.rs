use crate::cli::{Cli, ColorMode, Command, LinkMode, ThemeMode};
use rustdown::{LinkStyle, RenderOptions, Theme, render_markdown_with_options};
use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::{env, fs};

pub(crate) fn run(cli: Cli) -> io::Result<()> {
    let (raw_content, is_html) = match cli.command {
        Some(Command::Html { input }) => (read_input(input)?, true),
        None => (read_input(cli.input)?, false),
    };
    let markdown = if is_html {
        html2md::parse_html(&raw_content)
    } else {
        raw_content
    };
    let is_terminal = io::stdout().is_terminal();
    let color = match cli.color {
        ColorMode::Always => termcolor::ColorChoice::Always,
        ColorMode::Never => termcolor::ColorChoice::Never,
        ColorMode::Auto if !is_terminal || env::var_os("NO_COLOR").is_some() => {
            termcolor::ColorChoice::Never
        }
        ColorMode::Auto => termcolor::ColorChoice::Auto,
    };
    let width = cli
        .width
        .map(usize::from)
        .unwrap_or_else(|| {
            if is_terminal {
                terminal_size::terminal_size()
                    .map(|(terminal_size::Width(w), _)| usize::from(w))
                    .filter(|w| *w > 0)
                    .unwrap_or(80)
            } else {
                80
            }
        })
        .min(10_000);
    let options = RenderOptions {
        width,
        preserve_fences: cli.preserve_fences || env::var_os("RUSTDOWN_PRESERVE_FENCES").is_some(),
        wrap_code: cli.wrap_code,
        ascii: cli.ascii,
        links: match cli.links {
            LinkMode::Inline => LinkStyle::Inline,
            LinkMode::Text => LinkStyle::Text,
            LinkMode::Hyperlink => LinkStyle::Hyperlink,
            LinkMode::Reference => LinkStyle::Reference,
        },
        syntax_highlighting: cli.highlight,
        theme: match cli.theme {
            ThemeMode::Dark => Theme::Dark,
            ThemeMode::Light => Theme::Light,
        },
    };
    let mut output = termcolor::StandardStream::stdout(color);
    render_markdown_with_options(&markdown, &options, &mut output)
}

fn read_input(input: Option<String>) -> io::Result<String> {
    match input {
        Some(input) => {
            let path = Path::new(&input);
            match fs::read_to_string(path) {
                Ok(content) => Ok(content),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(input),
                Err(e) => Err(e),
            }
        }
        None => {
            let stdin = io::stdin();
            if stdin.is_terminal() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "No input provided. Usage: rustdown <file|string> or pipe via stdin.",
                ));
            }
            let mut buf = String::new();
            stdin.lock().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}
