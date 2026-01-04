use clap::Parser;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser as MdParser, Tag};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

struct MarkdownRenderer {
    output: StandardStream,
    // State tracking
    heading_level: Option<u32>,
    heading_buf: String,
    list_depth: usize,
    list_item_number: Vec<usize>,
    current_list_is_ordered: Vec<bool>,
    // Track whether the last written character was whitespace/newline
    last_was_space: bool,
    in_emphasis: bool,
    in_strong: bool,
    in_strikethrough: bool,
    in_code_block: bool,
    in_blockquote: bool,
    in_link: bool,
    link_url: String,
    in_image: bool,
    image_alt: String,
    image_url: String,
    // buffer for code blocks
    code_buf: String,
    code_lang: Option<String>,
    preserve_code_fences: bool,
    // table rendering state
    in_table: bool,
    in_table_head: bool,
    current_cell: String,
    current_row: Vec<String>,
    // collected table rows while inside a table
    table_rows: Vec<Vec<String>>,
    table_header_count: usize,
}

impl MarkdownRenderer {
    fn new() -> Self {
        Self {
            output: StandardStream::stdout(ColorChoice::Auto),
            heading_level: None,
            heading_buf: String::new(),
            list_depth: 0,
            list_item_number: Vec::new(),
            current_list_is_ordered: Vec::new(),
            last_was_space: true,
            in_emphasis: false,
            in_strong: false,
            in_strikethrough: false,
            in_code_block: false,
            in_blockquote: false,
            in_link: false,
            link_url: String::new(),
            in_image: false,
            image_alt: String::new(),
            image_url: String::new(),
            code_buf: String::new(),
            code_lang: None,
            preserve_code_fences: std::env::var("RUSTDOWN_PRESERVE_FENCES").is_ok(),
            in_table: false,
            in_table_head: false,
            current_cell: String::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_header_count: 0,
        }
    }

    fn render_heading(&mut self, level: u32, text: &str) -> io::Result<()> {
        writeln!(self.output)?; // Add spacing before heading
        match level {
            1 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Cyan))
                    .set_intense(true)
                    .set_bold(true);
                self.output.set_color(&spec)?;
                writeln!(self.output, "{}", text)?;
                self.output.reset()?;
                writeln!(self.output, "{}", "=".repeat(text.chars().count()))?;
            }
            2 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Blue))
                    .set_intense(true)
                    .set_bold(true);
                self.output.set_color(&spec)?;
                writeln!(self.output, "{}", text)?;
                self.output.reset()?;
                writeln!(self.output, "{}", "-".repeat(text.chars().count()))?;
            }
            3 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Green)).set_bold(true);
                self.output.set_color(&spec)?;
                writeln!(self.output, "### {}", text)?;
                self.output.reset()?;
            }
            4 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Yellow)).set_bold(true);
                self.output.set_color(&spec)?;
                writeln!(self.output, "#### {}", text)?;
                self.output.reset()?;
            }
            5 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Magenta));
                self.output.set_color(&spec)?;
                writeln!(self.output, "##### {}", text)?;
                self.output.reset()?;
            }
            6 => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::White)).set_intense(false);
                self.output.set_color(&spec)?;
                writeln!(self.output, "###### {}", text)?;
                self.output.reset()?;
            }
            _ => {
                writeln!(self.output, "{}", text)?;
            }
        }
        writeln!(self.output)?; // Add spacing after heading
        Ok(())
    }

    fn write_text(&mut self, text: &str) -> io::Result<()> {
        if self.heading_level.is_some() {
            self.heading_buf.push_str(text);
            return Ok(());
        }

        if self.in_image {
            self.image_alt.push_str(text);
            return Ok(());
        }

        // Handle table cell text with formatting preservation
        if self.in_table {
            self.current_cell.push_str(text);
            return Ok(());
        }

        let mut spec = ColorSpec::new();

        if self.in_strong {
            spec.set_bold(true);
        }
        if self.in_emphasis {
            spec.set_italic(true);
        }
        if self.in_strikethrough {
            spec.set_strikethrough(true);
        }
        if self.in_link {
            spec.set_fg(Some(Color::Blue)).set_underline(true);
        }
        if self.in_blockquote {
            spec.set_fg(Some(Color::Yellow));
        }

        self.output.set_color(&spec)?;
        write!(self.output, "{}", text)?;
        self.output.reset()?;

        // Update trailing-space tracking
        self.last_was_space = text
            .chars()
            .rev()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false);

        Ok(())
    }

    fn render_list_item_start(&mut self) -> io::Result<()> {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));

        let is_ordered = self
            .current_list_is_ordered
            .get(self.list_depth.saturating_sub(1))
            .copied()
            .unwrap_or(false);

        // Ensure previous content ended on its own line before the list marker
        if !self.last_was_space {
            writeln!(self.output)?;
        }

        if is_ordered {
            if self.list_depth > self.list_item_number.len() {
                self.list_item_number.push(1);
            } else if self.list_depth > 0 {
                self.list_item_number[self.list_depth - 1] += 1;
            }
            let num = self
                .list_item_number
                .get(self.list_depth.saturating_sub(1))
                .copied()
                .unwrap_or(1);
            write!(self.output, "{}{}. ", indent, num)?;
        } else {
            // Improved CommonMark-compatible bullet selection
            let bullet = match (self.list_depth.saturating_sub(1)) % 3 {
                0 => "-", // Use dash for top level (more CommonMark compatible)
                1 => "*", // Asterisk for second level
                _ => "+", // Plus for third level and beyond
            };
            write!(self.output, "{}{} ", indent, bullet)?;
        }
        self.last_was_space = false;
        Ok(())
    }

    fn render_inline_code(&mut self, code: &str) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red));
        self.output.set_color(&spec)?;
        write!(self.output, "`{}`", code)?;
        self.output.reset()?;
        self.last_was_space = false;
        Ok(())
    }

    fn render_image(&mut self, url: &str, alt: &str) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Magenta));
        self.output.set_color(&spec)?;
        write!(self.output, "🖼️  [IMAGE: {}]", alt)?;
        self.output.reset()?;

        let mut url_spec = ColorSpec::new();
        url_spec.set_fg(Some(Color::Cyan));
        self.output.set_color(&url_spec)?;
        write!(self.output, " ({})", url)?;
        self.output.reset()?;
        self.last_was_space = false;
        Ok(())
    }

    fn render_blockquote_start(&mut self) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow));
        self.output.set_color(&spec)?;
        // ensure quote starts on a new line
        if !self.last_was_space {
            writeln!(self.output)?;
        }
        write!(self.output, "│ ")?;
        self.output.reset()?;
        self.last_was_space = false;
        Ok(())
    }

    fn render_rule(&mut self) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::White)).set_intense(false);
        self.output.set_color(&spec)?;
        writeln!(self.output, "{}", "─".repeat(60))?;
        self.output.reset()?;
        self.last_was_space = true;
        Ok(())
    }

    fn render_task_list_item(&mut self, checked: bool) -> io::Result<()> {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));
        let checkbox = if checked { "☑" } else { "☐" };
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(if checked { Color::Green } else { Color::White }));
        self.output.set_color(&spec)?;
        write!(self.output, "{}{} ", indent, checkbox)?;
        self.output.reset()?;
        Ok(())
    }

    fn render_code_block(&mut self, code: &str, lang: Option<&str>) -> io::Result<()> {
        writeln!(self.output)?;

        if self.preserve_code_fences {
            // Show original fenced block with backticks
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Green)).set_bold(true);
            self.output.set_color(&spec)?;
            writeln!(self.output, "```{}", lang.unwrap_or(""))?;
            self.output.reset()?;

            // Print code without additional indentation
            for line in code.lines() {
                writeln!(self.output, "{}", line)?;
            }

            self.output.set_color(&spec)?;
            writeln!(self.output, "```")?;
            self.output.reset()?;
        } else {
            // Original rendering with language label and indentation
            if let Some(l) = lang {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Green)).set_bold(true);
                self.output.set_color(&spec)?;
                writeln!(self.output, "[{}]", l)?;
                self.output.reset()?;
            }
            for line in code.lines() {
                write!(self.output, "    {}\n", line)?;
            }
        }

        writeln!(self.output)?;
        self.last_was_space = true;
        Ok(())
    }

    fn render_table(&mut self) -> io::Result<()> {
        if self.table_rows.is_empty() {
            return Ok(());
        }

        // Compute column count and widths with proper cell trimming
        let cols = self.table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut widths = vec![0usize; cols];

        // Trim cells and calculate proper widths
        let mut trimmed_rows = Vec::new();
        for row in &self.table_rows {
            let mut trimmed_row = Vec::new();
            for (i, cell) in row.iter().enumerate() {
                let trimmed = cell.trim();
                trimmed_row.push(trimmed.to_string());
                if i < widths.len() {
                    widths[i] = widths[i].max(trimmed.chars().count());
                }
            }
            trimmed_rows.push(trimmed_row);
        }

        // Ensure minimum width for readability
        for width in &mut widths {
            *width = (*width).max(3);
        }

        // Print rows with improved alignment
        for (ri, row) in trimmed_rows.iter().enumerate() {
            let mut out_cells: Vec<String> = Vec::new();
            for i in 0..cols {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                let padded = format!(" {:<width$} ", cell, width = widths[i]);
                out_cells.push(padded);
            }
            writeln!(self.output, "|{}|", out_cells.join("|"))?;

            // After header row, print separator with proper alignment
            if self.table_header_count > 0 && ri + 1 == self.table_header_count {
                let mut sep_cells: Vec<String> = Vec::new();
                for &width in &widths {
                    sep_cells.push(format!(" {} ", "-".repeat(width)));
                }
                writeln!(self.output, "|{}|", sep_cells.join("|"))?;
            }
        }

        Ok(())
    }
}

fn render_markdown_to_terminal(md: &str) -> io::Result<()> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MdParser::new_ext(md, options);
    let mut renderer = MarkdownRenderer::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
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
                        renderer.heading_level = Some(num);
                        renderer.heading_buf.clear();
                    }
                    Tag::List(start_number) => {
                        // Ensure list starts on a new line
                        if !renderer.last_was_space {
                            writeln!(renderer.output)?;
                        }
                        let is_ordered = start_number.is_some();
                        renderer.current_list_is_ordered.push(is_ordered);
                        if is_ordered && renderer.list_item_number.len() <= renderer.list_depth {
                            renderer.list_item_number.push(0);
                        }
                        renderer.list_depth += 1;
                    }
                    Tag::Item => {
                        renderer.render_list_item_start()?;
                    }
                    Tag::Emphasis => {
                        renderer.in_emphasis = true;
                    }
                    Tag::Strong => {
                        renderer.in_strong = true;
                    }
                    Tag::Strikethrough => {
                        renderer.in_strikethrough = true;
                    }
                    Tag::Link { dest_url, .. } => {
                        renderer.in_link = true;
                        renderer.link_url = dest_url.to_string();
                    }
                    Tag::Image { dest_url, .. } => {
                        renderer.in_image = true;
                        renderer.image_url = dest_url.to_string();
                        renderer.image_alt.clear();
                    }
                    Tag::CodeBlock(kind) => {
                        renderer.in_code_block = true;
                        renderer.code_buf.clear();
                        // capture language for fenced blocks
                        renderer.code_lang = match kind {
                            CodeBlockKind::Fenced(info) => {
                                let s = info.split_whitespace().next().unwrap_or("").to_string();
                                if s.is_empty() {
                                    None
                                } else {
                                    Some(s)
                                }
                            }
                            CodeBlockKind::Indented => None,
                        };
                        // code block starts on its own line
                        if !renderer.last_was_space {
                            writeln!(renderer.output)?;
                        }
                        renderer.last_was_space = true;
                    }
                    Tag::BlockQuote(_) => {
                        renderer.in_blockquote = true;
                        renderer.render_blockquote_start()?;
                    }
                    Tag::Table(_) => {
                        renderer.in_table = true;
                        renderer.in_table_head = false;
                        // ensure table starts on its own line
                        if !renderer.last_was_space {
                            writeln!(renderer.output)?;
                        }
                        renderer.last_was_space = true;
                    }
                    Tag::TableHead => {
                        renderer.in_table_head = true;
                        renderer.current_row.clear();
                    }
                    Tag::TableRow => {
                        // Only clear current_row if we're not in a table head
                        // When in table head, the row was already cleared by TableHead start
                        if !renderer.in_table_head {
                            renderer.current_row.clear();
                        }
                    }
                    Tag::TableCell => {
                        renderer.current_cell.clear();
                    }
                    _ => {}
                }
            }
            Event::End(tag_end) => {
                match tag_end {
                    pulldown_cmark::TagEnd::TableCell => {
                        // finish cell
                        renderer.current_row.push(renderer.current_cell.clone());
                        renderer.current_cell.clear();
                    }
                    pulldown_cmark::TagEnd::TableRow => {
                        // store the row; rendering will happen at end of table
                        renderer.table_rows.push(renderer.current_row.clone());
                        renderer.current_row.clear();
                        renderer.last_was_space = true;
                    }
                    pulldown_cmark::TagEnd::TableHead => {
                        // Commit header row(s) if any cells are still buffered in current_row.
                        if !renderer.current_row.is_empty() {
                            renderer.table_rows.push(renderer.current_row.clone());
                            renderer.current_row.clear();
                        }
                        // Mark that we've finished processing the header
                        renderer.table_header_count = renderer.table_rows.len();
                        renderer.in_table_head = false;
                    }
                    pulldown_cmark::TagEnd::Table => {
                        // Use the improved table rendering method
                        renderer.render_table()?;
                        renderer.table_rows.clear();
                        renderer.table_header_count = 0;
                        renderer.in_table = false;
                        writeln!(renderer.output)?;
                        renderer.last_was_space = true;
                    }
                    pulldown_cmark::TagEnd::Paragraph => {
                        // end of paragraph: separate top-level paragraphs with a blank line,
                        // inside lists just emit a newline
                        if renderer.list_depth == 0 {
                            writeln!(renderer.output)?;
                            writeln!(renderer.output)?;
                            renderer.last_was_space = true;
                        } else {
                            writeln!(renderer.output)?;
                            renderer.last_was_space = true;
                        }
                    }
                    pulldown_cmark::TagEnd::Heading(_) => {
                        if let Some(level) = renderer.heading_level {
                            let heading_text = renderer.heading_buf.clone();
                            renderer.render_heading(level, &heading_text)?;
                        }
                        renderer.heading_level = None;
                        renderer.heading_buf.clear();
                    }
                    pulldown_cmark::TagEnd::List(_) => {
                        renderer.list_depth = renderer.list_depth.saturating_sub(1);
                        renderer.current_list_is_ordered.pop();
                        if renderer.current_list_is_ordered.is_empty() {
                            renderer.list_item_number.clear();
                        }
                        renderer.last_was_space = true;
                    }
                    pulldown_cmark::TagEnd::Item => {
                        writeln!(renderer.output)?;
                        renderer.last_was_space = true;
                    }
                    pulldown_cmark::TagEnd::Emphasis => {
                        renderer.in_emphasis = false;
                    }
                    pulldown_cmark::TagEnd::Strong => {
                        renderer.in_strong = false;
                    }
                    pulldown_cmark::TagEnd::Strikethrough => {
                        renderer.in_strikethrough = false;
                    }
                    pulldown_cmark::TagEnd::Link => {
                        if !renderer.link_url.is_empty() {
                            let mut url_spec = ColorSpec::new();
                            url_spec.set_fg(Some(Color::Cyan));
                            renderer.output.set_color(&url_spec)?;
                            write!(renderer.output, " ({})", renderer.link_url)?;
                            renderer.output.reset()?;
                        }
                        renderer.in_link = false;
                        renderer.link_url.clear();
                        renderer.last_was_space = false;
                    }
                    pulldown_cmark::TagEnd::Image => {
                        let image_url = renderer.image_url.clone();
                        let image_alt = renderer.image_alt.clone();
                        renderer.render_image(&image_url, &image_alt)?;
                        renderer.in_image = false;
                        renderer.image_url.clear();
                        renderer.image_alt.clear();
                        renderer.last_was_space = false;
                    }
                    pulldown_cmark::TagEnd::CodeBlock => {
                        // move the accumulated code buffer and language out so we don't hold
                        // an immutable borrow while calling a mutable method on renderer
                        let code = std::mem::take(&mut renderer.code_buf);
                        let lang_opt = renderer.code_lang.take();
                        renderer.render_code_block(&code, lang_opt.as_deref())?;
                        renderer.in_code_block = false;
                    }
                    pulldown_cmark::TagEnd::BlockQuote(_) => {
                        renderer.in_blockquote = false;
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if renderer.in_code_block {
                    // preserve exact code block content
                    renderer.code_buf.push_str(&text);
                } else if renderer.in_table {
                    // while in a table, accumulate into current cell
                    renderer.current_cell.push_str(&text);
                } else {
                    renderer.write_text(&text)?;
                }
            }
            Event::Code(code) => {
                if renderer.in_table {
                    // When in a table, accumulate inline code as text with backticks
                    renderer.current_cell.push('`');
                    renderer.current_cell.push_str(&code);
                    renderer.current_cell.push('`');
                } else if !renderer.in_code_block {
                    renderer.render_inline_code(&code)?;
                }
            }
            Event::Html(html) => {
                // Simple HTML handling - just print as-is for now
                write!(renderer.output, "{}", html)?;
            }
            Event::SoftBreak => {
                if renderer.heading_level.is_some() {
                    renderer.heading_buf.push(' ');
                } else if renderer.in_blockquote {
                    renderer.render_blockquote_start()?;
                } else {
                    write!(renderer.output, " ")?;
                    renderer.last_was_space = true;
                }
            }
            Event::HardBreak => {
                writeln!(renderer.output)?;
                renderer.last_was_space = true;
            }
            Event::Rule => {
                renderer.render_rule()?;
            }
            Event::FootnoteReference(name) => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Blue)).set_bold(true);
                renderer.output.set_color(&spec)?;
                write!(renderer.output, "[^{}]", name)?;
                renderer.output.reset()?;
            }
            Event::TaskListMarker(checked) => {
                renderer.render_task_list_item(checked)?;
            }
            _ => {}
        }
    }

    renderer.output.flush()?;
    Ok(())
}

fn main() -> io::Result<()> {
    #[derive(Parser, Debug)]
    #[command(
        name = "rustdown",
        about = "Render Markdown to the terminal with colors"
    )]
    struct Cli {
        /// Input Markdown file.
        file: PathBuf,

        /// Preserve original fenced code fences (like ```lang ... ```)
        #[arg(long = "preserve-fences")]
        preserve_fences: bool,
    }

    let cli = Cli::parse();

    if cli.preserve_fences {
        std::env::set_var("RUSTDOWN_PRESERVE_FENCES", "1");
    }

    let file_content = fs::read_to_string(cli.file);

    let markdown_content = match file_content {
        Ok(content) => content,
        Err(error) => panic!("There was a problem reading the file: {error:?}"),
    };

    render_markdown_to_terminal(&markdown_content)
}
