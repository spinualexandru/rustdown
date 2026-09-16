mod html;
mod inline;
mod layout;
mod table;

use crate::markdown::{Node, parse};
use layout::*;
use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, Tag};
use std::sync::OnceLock;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
};
use termcolor::Color;

/// How link destinations are displayed. Labels are always retained.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkStyle {
    #[default]
    Inline,
    Text,
    /// OSC 8 links, with inline URLs as fallback for unsupported writers or URI schemes.
    Hyperlink,
    /// Deduplicated numeric references and a Links section after the document.
    Reference,
}

/// Terminal background palette, including optional syntax highlighting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// Explicit, deterministic layout settings for embedding applications.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    /// Available terminal columns (1..=10000). Defaults to 80.
    pub width: usize,
    pub preserve_fences: bool,
    /// Wrap code, preserving spaces and expanding tabs at four-column stops.
    pub wrap_code: bool,
    /// Use ASCII decorations, without transliterating document content.
    pub ascii: bool,
    pub links: LinkStyle,
    /// Highlight fenced code with a recognized language. Defaults to false.
    pub syntax_highlighting: bool,
    pub theme: Theme,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 80,
            preserve_fences: false,
            wrap_code: false,
            ascii: false,
            links: LinkStyle::Inline,
            syntax_highlighting: false,
            theme: Theme::Dark,
        }
    }
}

pub(crate) struct Renderer<'a> {
    source: &'a str,
    options: &'a RenderOptions,
    html_depth: usize,
    references: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(source: &'a str, options: &'a RenderOptions) -> Self {
        Self {
            source,
            options,
            html_depth: 0,
            references: Default::default(),
        }
    }

    pub(crate) fn document(&self, nodes: &[Node<'_>]) -> Lines {
        let mut lines = self.sequence(nodes, self.options.width, false);
        let mut notes = Vec::new();
        self.notes(nodes, &mut notes);
        if !notes.is_empty() {
            join(
                &mut lines,
                wrap(
                    &[span(
                        "Notes",
                        Style {
                            bold: true,
                            ..Style::accent()
                        },
                    )],
                    self.options.width,
                ),
                true,
            );
            for (name, children) in notes {
                let label = span(format!("[^{name}]: "), Style::accent());
                let content = labeled(
                    label,
                    |columns| self.sequence(children, columns, false),
                    self.options.width,
                );
                join(&mut lines, content, true);
            }
        }
        let references = self.references.borrow();
        if !references.is_empty() {
            join(
                &mut lines,
                wrap(
                    &[span(
                        "Links",
                        Style {
                            bold: true,
                            ..Style::accent()
                        },
                    )],
                    self.options.width,
                ),
                true,
            );
            lines.push(Vec::new());
            for (i, uri) in references.iter().enumerate() {
                join(
                    &mut lines,
                    labeled(
                        span(format!("[{}]: ", i + 1), Style::accent()),
                        |columns| wrap(&[plain(uri)], columns),
                        self.options.width,
                    ),
                    false,
                );
            }
        }
        lines
    }

    fn notes<'n, 's>(&self, nodes: &'n [Node<'s>], out: &mut Vec<(String, &'n [Node<'s>])>) {
        for node in nodes {
            if let Node::Element(tag, children, _) = node {
                if let Tag::FootnoteDefinition(name) = tag {
                    out.push((name.to_string(), children));
                } else {
                    self.notes(children, out);
                }
            }
        }
    }

    fn sequence(&self, nodes: &[Node<'_>], columns: usize, compact: bool) -> Lines {
        let mut lines = Vec::new();
        let mut start = 0;
        for (i, node) in nodes.iter().enumerate() {
            if is_block(node) {
                if start < i {
                    join(
                        &mut lines,
                        wrap(&self.inline(&nodes[start..i], Style::default()), columns),
                        !compact,
                    );
                }
                join(&mut lines, self.block(node, columns), !compact);
                start = i + 1;
            }
        }
        if start < nodes.len() {
            join(
                &mut lines,
                wrap(&self.inline(&nodes[start..], Style::default()), columns),
                !compact,
            );
        }
        lines
    }

    fn block(&self, node: &Node<'_>, columns: usize) -> Lines {
        match node {
            Node::Event(Event::Rule) => vec![vec![span(
                if self.options.ascii { "-" } else { "─" }.repeat(columns),
                Style::accent(),
            )]],
            Node::Element(tag, children, range) => match tag {
                Tag::Paragraph => wrap(&self.inline(children, Style::default()), columns),
                Tag::Heading { level, .. } => {
                    let level = *level as u8;
                    let style = Style {
                        bold: level <= 4,
                        italic: level >= 5,
                        fg: if level <= 3 { Some(Color::Cyan) } else { None },
                        ..Style::default()
                    };
                    let mut lines = wrap(&self.inline(children, style), columns);
                    if level <= 2 {
                        let length = lines.iter().map(|l| width(l)).max().unwrap_or(0);
                        lines.push(vec![span(
                            if level == 1 { "=" } else { "-" }.repeat(length),
                            Style::accent(),
                        )]);
                    }
                    lines
                }
                Tag::List(start) => self.list(children, *start, columns),
                Tag::BlockQuote(kind) => {
                    let mut content = self.sequence(
                        children,
                        if columns >= 4 { columns - 2 } else { columns },
                        false,
                    );
                    let mut style = Style::muted();
                    if let Some(kind) = kind {
                        let label = match kind {
                            BlockQuoteKind::Note => "NOTE",
                            BlockQuoteKind::Tip => "TIP",
                            BlockQuoteKind::Important => "IMPORTANT",
                            BlockQuoteKind::Warning => "WARNING",
                            BlockQuoteKind::Caution => "CAUTION",
                        };
                        style.fg = Some(match kind {
                            BlockQuoteKind::Warning | BlockQuoteKind::Caution => Color::Yellow,
                            BlockQuoteKind::Tip => Color::Green,
                            _ => Color::Cyan,
                        });
                        let mut title = wrap(
                            &[span(
                                label,
                                Style {
                                    bold: true,
                                    ..style
                                },
                            )],
                            if columns >= 4 { columns - 2 } else { columns },
                        );
                        title.append(&mut content);
                        content = title;
                    }
                    let rail = span(if self.options.ascii { "> " } else { "│ " }, style);
                    if columns >= 4 {
                        prefix(content, rail.clone(), rail)
                    } else {
                        let mut lines = wrap(&[span(rail.text.trim_end(), rail.style)], columns);
                        lines.extend(content);
                        lines
                    }
                }
                Tag::Table(alignments) => self.table(children, alignments, columns),
                Tag::CodeBlock(kind) => self.code(children, kind, range.clone(), columns),
                Tag::FootnoteDefinition(_) => Vec::new(),
                Tag::HtmlBlock => {
                    let html: String = children
                        .iter()
                        .filter_map(|n| match n {
                            Node::Event(Event::Html(s)) => Some(s.as_ref()),
                            _ => None,
                        })
                        .collect();
                    let converted = crate::markdown::sanitize(&html2md::parse_html(&html));
                    if self.html_depth >= 4
                        || converted.trim() == html.trim()
                        || converted.trim().is_empty()
                    {
                        wrap(&[plain(html.trim_end())], columns)
                    } else {
                        let nodes = parse(&converted);
                        let renderer = Renderer {
                            source: &converted,
                            options: self.options,
                            html_depth: self.html_depth + 1,
                            references: self.references.clone(),
                        };
                        renderer.sequence(&nodes, columns, false)
                    }
                }
                _ => self.sequence(children, columns, false),
            },
            _ => Vec::new(),
        }
    }

    fn list(&self, items: &[Node<'_>], start: Option<u64>, columns: usize) -> Lines {
        let loose = items.iter().any(|n| matches!(n, Node::Element(Tag::Item, children, _) if children.iter().any(|c| matches!(c, Node::Element(Tag::Paragraph, _, _)))));
        let mut out = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let Node::Element(Tag::Item, children, _) = item else {
                continue;
            };
            let task = task_marker(children);
            let checkbox = task.map(|done| match (done, self.options.ascii) {
                (true, true) => "[x] ",
                (false, true) => "[ ] ",
                (true, false) => "☑ ",
                (false, false) => "☐ ",
            });
            let marker = if let Some(number) = start {
                format!(
                    "{}. {}",
                    number.saturating_add(index as u64),
                    checkbox.unwrap_or("")
                )
            } else {
                checkbox
                    .unwrap_or(if self.options.ascii { "- " } else { "• " })
                    .to_string()
            };
            let style = if task == Some(true) {
                Style {
                    fg: Some(Color::Green),
                    ..Style::default()
                }
            } else {
                Style::default()
            };
            join(
                &mut out,
                labeled(
                    span(marker, style),
                    |width| self.sequence(children, width, !loose),
                    columns,
                ),
                loose,
            );
        }
        out
    }

    fn code(
        &self,
        children: &[Node<'_>],
        kind: &CodeBlockKind<'_>,
        range: std::ops::Range<usize>,
        columns: usize,
    ) -> Lines {
        let code: String = children
            .iter()
            .filter_map(|node| match node {
                Node::Event(Event::Text(s)) => Some(s.as_ref()),
                _ => None,
            })
            .collect();
        let info = match kind {
            CodeBlockKind::Fenced(info) => info.as_ref(),
            CodeBlockKind::Indented => "",
        };
        let mut content = self.code_lines(&code, info.split_whitespace().next().unwrap_or(""));
        let preserved = self.options.preserve_fences && matches!(kind, CodeBlockKind::Fenced(_));
        let gutter = !preserved && columns >= 4;
        if self.options.wrap_code {
            content = content
                .iter()
                .flat_map(|line| wrap_code(line, if gutter { columns - 2 } else { columns }))
                .collect();
        }
        if preserved {
            let source = &self.source[range];
            let opener = source.lines().next().unwrap_or("```");
            let marker = opener.chars().next().unwrap_or('`');
            let count = opener.chars().take_while(|&c| c == marker).count();
            let mut lines = vec![vec![span(opener, Style::accent())]];
            lines.append(&mut content);
            if let Some(last) = source.lines().last() {
                let last = last.trim_start_matches([' ', '\t', '>']);
                let delimiter = last.trim_end();
                if source.lines().count() > 1
                    && delimiter.chars().count() >= count
                    && delimiter.chars().all(|c| c == marker)
                {
                    lines.push(vec![span(last, Style::accent())]);
                }
            }
            return lines;
        }
        let label = if info.is_empty() { "code" } else { info };
        let top = if self.options.ascii {
            format!("+- {label}")
        } else {
            format!("╭─ {label}")
        };
        let mut lines = wrap(&[span(top, Style::accent())], columns);
        let rail = span(
            if self.options.ascii { "| " } else { "│ " },
            Style::accent(),
        );
        if gutter {
            lines.extend(prefix(content, rail.clone(), rail));
        } else {
            lines.extend(content);
        }
        lines.extend(wrap(
            &[span(
                if self.options.ascii { "+-" } else { "╰─" },
                Style::accent(),
            )],
            columns,
        ));
        lines
    }

    fn code_lines(&self, code: &str, language: &str) -> Lines {
        static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
        static THEMES: OnceLock<ThemeSet> = OnceLock::new();
        if self.options.syntax_highlighting && !language.is_empty() {
            let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
            if let Some(syntax) = syntaxes.find_syntax_by_token(language) {
                let themes = THEMES.get_or_init(ThemeSet::load_defaults);
                let theme = if self.options.theme == Theme::Light {
                    "InspiredGitHub"
                } else {
                    "base16-ocean.dark"
                };
                let mut highlight = HighlightLines::new(syntax, &themes.themes[theme]);
                return code
                    .split_inclusive('\n')
                    .map(|line| match highlight.highlight_line(line, syntaxes) {
                        Ok(ranges) => ranges
                            .into_iter()
                            .map(|(s, value)| {
                                span(
                                    value.trim_end_matches('\n'),
                                    Style {
                                        fg: Some(Color::Rgb(
                                            s.foreground.r,
                                            s.foreground.g,
                                            s.foreground.b,
                                        )),
                                        bold: s.font_style.contains(FontStyle::BOLD),
                                        italic: s.font_style.contains(FontStyle::ITALIC),
                                        underline: s.font_style.contains(FontStyle::UNDERLINE),
                                        ..Style::default()
                                    },
                                )
                            })
                            .collect(),
                        Err(_) => vec![plain(line.trim_end_matches('\n'))],
                    })
                    .collect();
            }
        }
        code.lines().map(|line| vec![plain(line)]).collect()
    }
}

fn is_block(node: &Node<'_>) -> bool {
    matches!(
        node,
        Node::Event(Event::Rule)
            | Node::Element(
                Tag::Paragraph
                    | Tag::Heading { .. }
                    | Tag::List(_)
                    | Tag::BlockQuote(_)
                    | Tag::CodeBlock(_)
                    | Tag::Table(_)
                    | Tag::FootnoteDefinition(_)
                    | Tag::HtmlBlock,
                _,
                _
            )
    )
}
fn task_marker(nodes: &[Node<'_>]) -> Option<bool> {
    match nodes.first()? {
        Node::Event(Event::TaskListMarker(checked)) => Some(*checked),
        Node::Element(Tag::Paragraph, children, _) => task_marker(children),
        _ => None,
    }
}
