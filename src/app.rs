use crate::cli::{Cli, Command};
use crate::markdown::render_markdown;
use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::{env, fs};

pub(crate) fn run(cli: Cli) -> io::Result<()> {
    let (raw_content, is_html, preserve_fences) = match cli.command {
        Some(Command::Html {
            input,
            preserve_fences,
        }) => {
            let pf = preserve_fences || env::var_os("RUSTDOWN_PRESERVE_FENCES").is_some();
            let content = read_input(input)?;
            (content, true, pf)
        }
        None => {
            let pf = cli.preserve_fences || env::var_os("RUSTDOWN_PRESERVE_FENCES").is_some();
            let content = read_input(cli.input)?;
            let is_html = looks_like_html(&content);
            (content, is_html, pf)
        }
    };

    let markdown_content = if is_html {
        html2md::parse_html(&raw_content)
    } else {
        raw_content
    };

    let mut output = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
    render_markdown(&markdown_content, preserve_fences, &mut output)
}

/// Heuristic: content is likely HTML if it starts with an HTML tag or contains
/// block-level HTML elements that wouldn't normally appear in plain Markdown.
fn looks_like_html(content: &str) -> bool {
    let trimmed = content.trim();
    // Starts with any HTML tag
    if trimmed.starts_with('<') && !trimmed.starts_with("<<") {
        // Check it's actually a tag, not just a less-than sign
        if let Some(end) = trimmed.find('>') {
            let tag_content = &trimmed[1..end];
            let tag_name = tag_content
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('/');
            let html_tags = [
                "html", "head", "body", "div", "span", "p", "a", "b", "i", "u", "em",
                "strong", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "table",
                "tr", "td", "th", "thead", "tbody", "br", "hr", "img", "section", "article",
                "nav", "header", "footer", "main", "form", "input", "button", "select",
                "textarea", "label", "pre", "code", "blockquote", "dl", "dt", "dd",
                "!doctype",
            ];
            return html_tags
                .iter()
                .any(|t| tag_name.eq_ignore_ascii_case(t));
        }
    }
    false
}

fn read_input(input: Option<String>) -> io::Result<String> {
    match input {
        Some(input) => {
            let path = Path::new(&input);
            if path.exists() {
                fs::read_to_string(path)
            } else {
                Ok(input)
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
