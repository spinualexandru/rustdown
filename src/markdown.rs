use crate::render::MarkdownRenderer;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser as MdParser, Tag};
use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

pub fn render_markdown(
    md: &str,
    preserve_fences: bool,
    output: &mut impl WriteColor,
) -> io::Result<()> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MdParser::new_ext(md, options);
    let mut renderer = MarkdownRenderer::new(output, preserve_fences);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    // nothing to do at paragraph start
                }
                Tag::Heading { level, .. } => {
                    let num = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    renderer.start_heading(num);
                }
                Tag::List(start_number) => {
                    renderer.start_list(start_number)?;
                }
                Tag::Item => {
                    renderer.render_list_item_start()?;
                }
                Tag::Emphasis => renderer.set_emphasis(true),
                Tag::Strong => renderer.set_strong(true),
                Tag::Strikethrough => renderer.set_strikethrough(true),
                Tag::Link { dest_url, .. } => renderer.start_link(&dest_url),
                Tag::Image { dest_url, .. } => renderer.start_image(&dest_url),
                Tag::CodeBlock(kind) => {
                    renderer.start_code_block(match kind {
                        CodeBlockKind::Fenced(info) => pulldown_cmark::CodeBlockKind::Fenced(info),
                        CodeBlockKind::Indented => pulldown_cmark::CodeBlockKind::Indented,
                    })?;
                }
                Tag::BlockQuote(_) => {
                    renderer.start_blockquote()?;
                }
                Tag::Table(_) => {
                    renderer.start_table()?;
                }
                Tag::TableHead => renderer.start_table_head(),
                Tag::TableRow => renderer.start_table_row(),
                Tag::TableCell => renderer.start_table_cell(),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                pulldown_cmark::TagEnd::TableCell => renderer.end_table_cell(),
                pulldown_cmark::TagEnd::TableRow => renderer.end_table_row(),
                pulldown_cmark::TagEnd::TableHead => renderer.end_table_head(),
                pulldown_cmark::TagEnd::Table => renderer.end_table()?,
                pulldown_cmark::TagEnd::Paragraph => renderer.end_paragraph()?,
                pulldown_cmark::TagEnd::Heading(_) => renderer.end_heading()?,
                pulldown_cmark::TagEnd::List(_) => renderer.end_list(),
                pulldown_cmark::TagEnd::Item => renderer.end_item()?,
                pulldown_cmark::TagEnd::Emphasis => renderer.set_emphasis(false),
                pulldown_cmark::TagEnd::Strong => renderer.set_strong(false),
                pulldown_cmark::TagEnd::Strikethrough => renderer.set_strikethrough(false),
                pulldown_cmark::TagEnd::Link => renderer.end_link()?,
                pulldown_cmark::TagEnd::Image => renderer.end_image()?,
                pulldown_cmark::TagEnd::CodeBlock => renderer.end_code_block()?,
                pulldown_cmark::TagEnd::BlockQuote(_) => renderer.end_blockquote(),
                _ => {}
            },
            Event::Text(text) => renderer.write_event_text(&text)?,
            Event::Code(code) => renderer.write_event_code(&code)?,
            Event::Html(html) => {
                write!(renderer.output, "{}", html)?;
            }
            Event::SoftBreak => renderer.soft_break()?,
            Event::HardBreak => renderer.hard_break()?,
            Event::Rule => renderer.render_rule()?,
            Event::FootnoteReference(name) => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Blue)).set_bold(true);
                renderer.output.set_color(&spec)?;
                write!(renderer.output, "[^{}]", name)?;
                renderer.output.reset()?;
            }
            Event::TaskListMarker(checked) => renderer.render_task_list_item(checked)?,
            _ => {}
        }
    }

    renderer.flush()?;
    Ok(())
}
