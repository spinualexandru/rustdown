use crate::cli::Cli;
use crate::markdown::render_markdown;
use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::{env, fs};

pub(crate) fn run(cli: Cli) -> io::Result<()> {
    let preserve_fences = cli.preserve_fences || env::var_os("RUSTDOWN_PRESERVE_FENCES").is_some();

    let markdown_content = match cli.input {
        Some(input) => {
            let path = Path::new(&input);
            if path.exists() {
                fs::read_to_string(path)?
            } else {
                input
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
            buf
        }
    };

    let mut output = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
    render_markdown(&markdown_content, preserve_fences, &mut output)
}
