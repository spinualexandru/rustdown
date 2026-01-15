use crate::cli::Cli;
use crate::markdown::render_markdown;
use std::{env, fs, io};

pub(crate) fn run(cli: Cli) -> io::Result<()> {
    // Backwards compatible: allow enabling via external env var, but don't mutate env at runtime
    let preserve_fences = cli.preserve_fences || env::var_os("RUSTDOWN_PRESERVE_FENCES").is_some();

    let file_content = fs::read_to_string(cli.file);

    let markdown_content = match file_content {
        Ok(content) => content,
        Err(error) => panic!("There was a problem reading the file: {error:?}"),
    };

    let mut output = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
    render_markdown(&markdown_content, preserve_fences, &mut output)
}
