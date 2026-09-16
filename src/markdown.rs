use pulldown_cmark::{Event, Options, Parser, Tag};
use std::{io, ops::Range};
use termcolor::WriteColor;

use crate::render::{RenderOptions, Renderer};

pub(crate) enum Node<'a> {
    Element(Tag<'a>, Vec<Node<'a>>, Range<usize>),
    Event(Event<'a>),
}

pub(crate) fn parse(md: &str) -> Vec<Node<'static>> {
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    // Byte-for-byte offsets remain valid: only suffix tabs on fence-shaped lines
    // become spaces in the parser view. Text/code/HTML are restored below.
    let normalized = normalize_fence_tabs(md);
    let mut roots = Vec::new();
    let mut stack: Vec<(Tag<'static>, Vec<Node<'static>>, usize)> = Vec::new();
    for (event, range) in Parser::new_ext(&normalized, options).into_offset_iter() {
        let event = restore_event(event, &range, md, &normalized);
        let node = match event {
            Event::Start(tag) => {
                stack.push((tag, Vec::new(), range.start));
                continue;
            }
            Event::End(_) => {
                let (tag, children, start) = stack.pop().expect("balanced parser events");
                Node::Element(tag, children, start..range.end)
            }
            event => Node::Event(event),
        };
        if let Some((_, children, _)) = stack.last_mut() {
            children.push(node);
        } else {
            roots.push(node);
        }
    }
    roots
}

/// Work around pulldown-cmark 0.13.4's space-only closing-fence scanner.
fn normalize_fence_tabs(md: &str) -> String {
    let mut bytes = md.as_bytes().to_vec();
    let mut offset = 0;
    for line in md.split_inclusive('\n') {
        let candidate = line.trim_start_matches([' ', '\t', '>']);
        let marker = candidate.as_bytes().first().copied().unwrap_or(0);
        let count = candidate.bytes().take_while(|b| *b == marker).count();
        if matches!(marker, b'`' | b'~') && count >= 3 {
            let suffix = &candidate[count..];
            if suffix.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\n')) {
                let start = offset + line.len() - suffix.len();
                for byte in &mut bytes[start..offset + line.len()] {
                    if *byte == b'\t' {
                        *byte = b' ';
                    }
                }
            }
        }
        offset += line.len();
    }
    String::from_utf8(bytes).expect("only ASCII tabs replaced")
}

fn restore_event(
    event: Event<'_>,
    range: &Range<usize>,
    original: &str,
    normalized: &str,
) -> Event<'static> {
    if original[range.clone()] == normalized[range.clone()] {
        return event.into_static();
    }
    match event {
        Event::Text(value) if value.as_ref() == &normalized[range.clone()] => {
            Event::Text(original[range.clone()].to_owned().into())
        }
        Event::Html(_) => Event::Html(original[range.clone()].to_owned().into()),
        Event::InlineHtml(_) => Event::InlineHtml(original[range.clone()].to_owned().into()),
        // Code spans have their own whitespace rules; parse the untouched span.
        Event::Code(value) => Parser::new(&format!("x {}", &original[range.clone()]))
            .find_map(|e| {
                if let Event::Code(code) = e {
                    Some(Event::Code(code.into_static()))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| Event::Code(value.into_static())),
        event => event.into_static(),
    }
}

/// Remove terminal control sequences before parsing, including those in code and URLs.
/// Line feeds and tabs are retained; a carriage return cannot reposition the cursor.
pub(crate) fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' | '\u{009b}' | '\u{009d}' | '\u{0090}' | '\u{009e}' | '\u{009f}' => {
                let introducer = match c {
                    '\x1b' => chars.next(),
                    '\u{009b}' => Some('['),
                    '\u{009d}' => Some(']'),
                    _ => Some('P'),
                };
                match introducer {
                    Some('[') => {
                        for c in chars.by_ref() {
                            if ('\u{0040}'..='\u{007e}').contains(&c) {
                                break;
                            }
                        }
                    }
                    Some(']' | 'P' | '^' | '_' | 'X') => {
                        let mut escape = false;
                        for c in chars.by_ref() {
                            if c == '\x07' || c == '\u{009c}' || (escape && c == '\\') {
                                break;
                            }
                            escape = c == '\x1b';
                        }
                    }
                    Some(c) if (' '..='/').contains(&c) => {
                        for c in chars.by_ref() {
                            if ('0'..='~').contains(&c) {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            _ => out.push(c),
        }
    }
    out
}

/// Render Markdown with the default layout and an optional original-fence display.
pub fn render_markdown(
    md: &str,
    preserve_fences: bool,
    output: &mut impl WriteColor,
) -> io::Result<()> {
    render_with_options(
        md,
        &RenderOptions {
            preserve_fences,
            ..RenderOptions::default()
        },
        output,
    )
}

pub(crate) fn render_with_options(
    md: &str,
    options: &RenderOptions,
    output: &mut impl WriteColor,
) -> io::Result<()> {
    if !(1..=10_000).contains(&options.width) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "render width must be between 1 and 10000 columns",
        ));
    }
    let mut options = options.clone();
    if options.links == crate::render::LinkStyle::Hyperlink && !output.supports_hyperlinks() {
        options.links = crate::render::LinkStyle::Inline;
    }
    let clean = sanitize(md);
    let nodes = parse(&clean);
    let renderer = Renderer::new(&clean, &options);
    let lines = renderer.document(&nodes);
    for line in lines {
        for span in line {
            output.set_color(&span.style.spec(options.theme))?;
            if let Some(uri) = &span.link {
                output.set_hyperlink(&termcolor::HyperlinkSpec::open(uri.as_bytes()))?;
            }
            let written = write!(output, "{}", span.text);
            // Close even after a failed write so a recoverable writer cannot leak a link.
            let closed = if span.link.is_some() {
                output.set_hyperlink(&termcolor::HyperlinkSpec::close())
            } else {
                Ok(())
            };
            written?;
            closed?;
        }
        output.reset()?;
        writeln!(output)?;
    }
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_adapter_restores_multiline_inline_code_tabs() {
        let nodes = parse("prefix ````a\n    ```\t\nb```` suffix");
        let Node::Element(Tag::Paragraph, children, _) = &nodes[0] else {
            panic!("expected paragraph")
        };
        let code = children.iter().find_map(|n| match n {
            Node::Event(Event::Code(s)) => Some(s.as_ref()),
            _ => None,
        });
        assert_eq!(code, Some("a     ```\t b"));
    }
}
