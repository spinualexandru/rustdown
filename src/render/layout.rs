use termcolor::{Color, ColorSpec};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub fg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
}

impl Style {
    pub fn spec(self, theme: super::Theme) -> ColorSpec {
        let dark = theme == super::Theme::Dark;
        let fg = self.fg.map(|color| match (color, dark) {
            (Color::Cyan, true) => Color::Rgb(103, 209, 223),
            (Color::Cyan, false) => Color::Rgb(0, 91, 117),
            (Color::Green, true) => Color::Rgb(145, 201, 133),
            (Color::Green, false) => Color::Rgb(26, 112, 50),
            (Color::Yellow, true) => Color::Rgb(229, 185, 92),
            (Color::Yellow, false) => Color::Rgb(130, 86, 0),
            (Color::Ansi256(8), true) => Color::Rgb(105, 123, 145),
            (Color::Ansi256(8), false) => Color::Rgb(104, 115, 126),
            (color, _) => color,
        });
        let mut spec = ColorSpec::new();
        spec.set_fg(fg)
            .set_bold(self.bold)
            .set_italic(self.italic)
            .set_strikethrough(self.strike)
            .set_underline(self.underline);
        spec
    }
    pub fn muted() -> Self {
        Self {
            fg: Some(Color::Ansi256(8)),
            ..Self::default()
        }
    }
    pub fn accent() -> Self {
        Self {
            fg: Some(Color::Cyan),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Span {
    pub text: String,
    pub style: Style,
    pub link: Option<std::sync::Arc<str>>,
}
pub(crate) type Line = Vec<Span>;
pub(crate) type Lines = Vec<Line>;

pub(crate) fn span(text: impl Into<String>, style: Style) -> Span {
    // Parsers may decode numeric HTML entities into controls after input sanitation.
    let mut text = text.into();
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        text = crate::markdown::sanitize(&text);
    }
    Span {
        text,
        style,
        link: None,
    }
}
pub(crate) fn plain(text: impl Into<String>) -> Span {
    span(text, Style::default())
}
pub(crate) fn width(line: &[Span]) -> usize {
    line.iter().map(|s| s.text.width()).sum()
}
pub(crate) fn text(line: &[Span]) -> String {
    line.iter().map(|s| s.text.as_str()).collect()
}
pub(crate) fn push(line: &mut Line, value: Span) {
    if value.text.is_empty() {
        return;
    }
    if let Some(last) = line.last_mut()
        && last.style == value.style
        && last.link == value.link
    {
        last.text.push_str(&value.text);
    } else {
        line.push(value);
    }
}

/// Wrap complete styled words, splitting oversized words only at grapheme boundaries.
/// Code blocks deliberately bypass this function to preserve their whitespace.
pub(crate) fn wrap(spans: &[Span], columns: usize) -> Lines {
    let columns = columns.max(1);
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut word = Vec::new();
    let mut spaces = Vec::new();
    fn flush_word(
        lines: &mut Lines,
        line: &mut Line,
        word: &mut Line,
        spaces: &mut Line,
        columns: usize,
    ) {
        if word.is_empty() {
            return;
        }
        if !line.is_empty() && width(line) + width(spaces) + width(word) > columns {
            lines.push(std::mem::take(line));
            spaces.clear();
        }
        if !line.is_empty() {
            for s in spaces.drain(..) {
                push(line, s);
            }
        } else {
            spaces.clear();
        }
        for s in word.drain(..) {
            for g in s.text.graphemes(true) {
                if !line.is_empty() && width(line) + g.width() > columns {
                    lines.push(std::mem::take(line));
                }
                push(line, fragment(g, &s));
            }
        }
    }
    for s in spans {
        for g in s.text.graphemes(true) {
            if g == "\n" {
                flush_word(&mut lines, &mut line, &mut word, &mut spaces, columns);
                spaces.clear();
                lines.push(std::mem::take(&mut line));
            } else if g.chars().all(char::is_whitespace) && g != "\u{00a0}" {
                flush_word(&mut lines, &mut line, &mut word, &mut spaces, columns);
                push(&mut spaces, fragment(if g == "\t" { " " } else { g }, s));
            } else {
                if g.width() > columns {
                    let escaped: String = g
                        .chars()
                        .map(|c| format!("\\u{{{:x}}}", c as u32))
                        .collect();
                    push(&mut word, fragment(&escaped, s));
                } else {
                    push(&mut word, fragment(g, s));
                }
            }
        }
    }
    flush_word(&mut lines, &mut line, &mut word, &mut spaces, columns);
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

pub(crate) fn join(into: &mut Lines, mut block: Lines, blank: bool) {
    if block.is_empty() {
        return;
    }
    if blank && !into.is_empty() {
        into.push(Vec::new());
    }
    into.append(&mut block);
}

pub(crate) fn prefix(lines: Lines, first: Span, rest: Span) -> Lines {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            line.insert(0, if i == 0 { first.clone() } else { rest.clone() });
            line
        })
        .collect()
}

/// Copy inline metadata when splitting a span for layout.
fn fragment(text: &str, source: &Span) -> Span {
    let mut result = span(text, source.style);
    result.link = source.link.clone();
    result
}

/// Wrap a code line without collapsing whitespace. Tabs use four-column stops.
pub(crate) fn wrap_code(line: &[Span], columns: usize) -> Lines {
    let columns = columns.max(1);
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut logical_column = 0;
    for s in line {
        for g in s.text.graphemes(true) {
            let expanded = if g == "\t" {
                " ".repeat(4 - logical_column % 4)
            } else if g.width() > columns {
                g.chars()
                    .map(|c| format!("\\u{{{:x}}}", c as u32))
                    .collect()
            } else {
                g.to_string()
            };
            logical_column += expanded.width();
            for g in expanded.graphemes(true) {
                if !current.is_empty() && width(&current) + g.width() > columns {
                    out.push(std::mem::take(&mut current));
                }
                push(&mut current, fragment(g, s));
            }
        }
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

/// Keep long labels readable without consuming all of the body width.
pub(crate) fn labeled(label: Span, body: impl FnOnce(usize) -> Lines, columns: usize) -> Lines {
    let indent = label.text.width();
    // A label that technically fits can still leave an unreadably thin body.
    let minimum_body = (columns / 2).clamp(2, 16);
    if columns >= indent.saturating_add(minimum_body) {
        let mut lines = body(columns - indent);
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        prefix(lines, label, plain(" ".repeat(indent)))
    } else {
        let mut lines = wrap(&[span(label.text.trim_end(), label.style)], columns);
        lines.extend(body(columns));
        lines
    }
}
