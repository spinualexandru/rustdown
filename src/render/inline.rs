use super::{LinkStyle, Renderer, html, layout::*};
use crate::markdown::Node;
use html5ever::tokenizer::TagKind;
use pulldown_cmark::{Event, Tag};
use termcolor::Color;

struct HtmlScope {
    name: String,
    previous: Style,
    start: usize,
    href: String,
}

impl Renderer<'_> {
    pub(super) fn inline(&self, nodes: &[Node<'_>], base: Style) -> Line {
        let mut out = Vec::new();
        let mut current = base;
        let mut scopes: Vec<HtmlScope> = Vec::new();
        for node in nodes {
            match node {
                Node::Event(Event::Text(value)) => push(&mut out, span(value.to_string(), current)),
                Node::Event(Event::Code(value)) => push(
                    &mut out,
                    span(
                        format!("`{value}`"),
                        Style {
                            fg: Some(Color::Cyan),
                            ..current
                        },
                    ),
                ),
                Node::Event(Event::SoftBreak) => push(&mut out, span(" ", current)),
                Node::Event(Event::HardBreak) => push(&mut out, span("\n", current)),
                Node::Event(Event::FootnoteReference(name)) => push(
                    &mut out,
                    span(
                        format!("[^{name}]"),
                        Style {
                            fg: Some(Color::Cyan),
                            ..current
                        },
                    ),
                ),
                Node::Event(Event::InlineHtml(raw)) => {
                    let Some(tag) = html::tag(raw) else {
                        if !raw.starts_with("<!--") {
                            push(&mut out, span(raw.to_string(), current));
                        }
                        continue;
                    };
                    let name = tag.name.as_ref();
                    if name == "br" {
                        push(&mut out, span("\n", current));
                        continue;
                    }
                    if name == "wbr" {
                        continue;
                    }
                    if name == "img" && tag.kind == TagKind::StartTag {
                        for s in
                            self.image(&html::attr(&tag, "src"), &html::attr(&tag, "alt"), current)
                        {
                            push(&mut out, s);
                        }
                        continue;
                    }
                    if !matches!(
                        name,
                        "a" | "b"
                            | "strong"
                            | "i"
                            | "em"
                            | "s"
                            | "del"
                            | "strike"
                            | "u"
                            | "ins"
                            | "code"
                            | "kbd"
                            | "samp"
                            | "var"
                            | "sup"
                            | "sub"
                            | "span"
                            | "small"
                            | "abbr"
                            | "mark"
                    ) {
                        push(&mut out, span(raw.to_string(), current));
                        continue;
                    }
                    if tag.kind == TagKind::EndTag {
                        if let Some(index) = scopes.iter().rposition(|scope| scope.name == name) {
                            let scope = scopes.remove(index);
                            scopes.truncate(index);
                            current = scope.previous;
                            if name == "a" {
                                let content = out.split_off(scope.start);
                                for s in self.link(content, &scope.href, current) {
                                    push(&mut out, s);
                                }
                            } else if matches!(name, "sup" | "sub") {
                                push(&mut out, span(")", current));
                            }
                        }
                    } else {
                        let scope = HtmlScope {
                            name: name.to_string(),
                            previous: current,
                            start: out.len(),
                            href: html::attr(&tag, "href"),
                        };
                        match name {
                            "b" | "strong" | "kbd" => current.bold = true,
                            "i" | "em" | "var" => current.italic = true,
                            "s" | "del" | "strike" => current.strike = true,
                            "u" | "ins" | "mark" => current.underline = true,
                            "a" => {
                                current.fg = Some(Color::Cyan);
                                current.underline = true;
                            }
                            "code" | "samp" => current.fg = Some(Color::Cyan),
                            "sup" => push(&mut out, span("^(", current)),
                            "sub" => push(&mut out, span("_(", current)),
                            _ => {}
                        }
                        // An explicit boundary keeps an HTML anchor separate from the previous span.
                        if name == "a" {
                            out.push(span("", current));
                        }
                        scopes.push(scope);
                    }
                }
                Node::Element(tag, children, _) => {
                    let mut style = current;
                    let content = match tag {
                        Tag::Strong => {
                            style.bold = true;
                            self.inline(children, style)
                        }
                        Tag::Emphasis => {
                            style.italic = true;
                            self.inline(children, style)
                        }
                        Tag::Strikethrough => {
                            style.strike = true;
                            self.inline(children, style)
                        }
                        Tag::Link { dest_url, .. } => {
                            style.fg = Some(Color::Cyan);
                            style.underline = true;
                            self.link(self.inline(children, style), dest_url, current)
                        }
                        Tag::Image { dest_url, .. } => {
                            let alt = text(&self.inline(children, current)).replace('\n', " ");
                            self.image(dest_url, &alt, current)
                        }
                        _ => self.inline(children, style),
                    };
                    for s in content {
                        push(&mut out, s);
                    }
                }
                _ => {}
            }
        }
        // HTML anchors without an end tag extend to the end of this inline block.
        while let Some(scope) = scopes.pop() {
            if scope.name == "a" {
                let content = out.split_off(scope.start);
                out.extend(self.link(content, &scope.href, scope.previous));
            } else if matches!(scope.name.as_str(), "sup" | "sub") {
                push(&mut out, span(")", scope.previous));
            }
        }
        out
    }

    fn image(&self, uri: &str, alt: &str, style: Style) -> Line {
        let label = if alt.is_empty() {
            "[image]".to_string()
        } else {
            format!("[image: {}]", alt.replace('\n', " "))
        };
        self.link(
            vec![span(
                label,
                Style {
                    fg: Some(Color::Cyan),
                    ..style
                },
            )],
            uri,
            style,
        )
    }

    fn link(&self, mut content: Line, uri: &str, style: Style) -> Line {
        let uri = crate::markdown::sanitize(uri);
        if uri.is_empty() {
            return content;
        }
        let label = text(&content);
        match self.options.links {
            LinkStyle::Text => {}
            LinkStyle::Reference => {
                let mut references = self.references.borrow_mut();
                let number = if let Some(index) = references.iter().position(|s| s == &uri) {
                    index + 1
                } else {
                    references.push(uri);
                    references.len()
                };
                push(
                    &mut content,
                    span(
                        format!("[{number}]"),
                        Style {
                            fg: Some(Color::Cyan),
                            ..style
                        },
                    ),
                );
            }
            LinkStyle::Hyperlink if hyperlink_uri(&uri).is_some() => {
                let destination: std::sync::Arc<str> = hyperlink_uri(&uri).unwrap().into();
                if label.is_empty() {
                    content.push(span(&uri, style));
                }
                for s in &mut content {
                    s.link = Some(destination.clone());
                }
            }
            LinkStyle::Inline | LinkStyle::Hyperlink => {
                if label != uri && label != uri.trim_start_matches("mailto:") {
                    push(
                        &mut content,
                        span(
                            format!(" ({uri})"),
                            Style {
                                fg: Some(Color::Cyan),
                                ..style
                            },
                        ),
                    );
                }
            }
        }
        content
    }
}

/// Only ordinary navigation targets become active links. Unsupported schemes stay visible.
fn hyperlink_uri(uri: &str) -> Option<String> {
    let (scheme, _) = uri.split_once(':')?;
    if !["https", "http", "mailto", "file"]
        .iter()
        .any(|s| scheme.eq_ignore_ascii_case(s))
        || uri.chars().any(char::is_control)
    {
        return None;
    }
    Some(uri.replace(' ', "%20"))
}
