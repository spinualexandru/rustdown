mod table;

use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

pub(crate) struct MarkdownRenderer<'a, W: WriteColor + ?Sized> {
    pub(crate) output: &'a mut W,
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

impl<'a, W: WriteColor + ?Sized> MarkdownRenderer<'a, W> {
    pub(crate) fn new(output: &'a mut W, preserve_code_fences: bool) -> Self {
        Self {
            output,
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
            preserve_code_fences,
            in_table: false,
            in_table_head: false,
            current_cell: String::new(),
            current_row: Vec::new(),
            table_rows: Vec::new(),
            table_header_count: 0,
        }
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }

    pub(crate) fn ensure_newline(&mut self) -> io::Result<()> {
        if !self.last_was_space {
            writeln!(self.output)?;
            self.last_was_space = true;
        }
        Ok(())
    }

    pub(crate) fn set_emphasis(&mut self, on: bool) {
        self.in_emphasis = on;
    }

    pub(crate) fn set_strong(&mut self, on: bool) {
        self.in_strong = on;
    }

    pub(crate) fn set_strikethrough(&mut self, on: bool) {
        self.in_strikethrough = on;
    }

    pub(crate) fn start_heading(&mut self, level: u32) {
        self.heading_level = Some(level);
        self.heading_buf.clear();
    }

    pub(crate) fn push_heading_softbreak(&mut self) {
        self.heading_buf.push(' ');
    }

    pub(crate) fn end_heading(&mut self) -> io::Result<()> {
        if let Some(level) = self.heading_level {
            let heading_text = self.heading_buf.clone();
            self.render_heading(level, &heading_text)?;
        }
        self.heading_level = None;
        self.heading_buf.clear();
        Ok(())
    }

    pub(crate) fn start_list(&mut self, start_number: Option<u64>) -> io::Result<()> {
        self.ensure_newline()?;

        let is_ordered = start_number.is_some();
        self.current_list_is_ordered.push(is_ordered);
        if is_ordered && self.list_item_number.len() <= self.list_depth {
            self.list_item_number.push(0);
        }
        self.list_depth += 1;
        Ok(())
    }

    pub(crate) fn end_list(&mut self) {
        self.list_depth = self.list_depth.saturating_sub(1);
        self.current_list_is_ordered.pop();
        if self.current_list_is_ordered.is_empty() {
            self.list_item_number.clear();
        }
        self.last_was_space = true;
    }

    pub(crate) fn end_item(&mut self) -> io::Result<()> {
        writeln!(self.output)?;
        self.last_was_space = true;
        Ok(())
    }

    pub(crate) fn start_link(&mut self, dest_url: &str) {
        self.in_link = true;
        self.link_url = dest_url.to_string();
    }

    pub(crate) fn end_link(&mut self) -> io::Result<()> {
        if !self.link_url.is_empty() {
            let mut url_spec = ColorSpec::new();
            url_spec.set_fg(Some(Color::Cyan));
            self.output.set_color(&url_spec)?;
            write!(self.output, " ({})", self.link_url)?;
            self.output.reset()?;
        }
        self.in_link = false;
        self.link_url.clear();
        self.last_was_space = false;
        Ok(())
    }

    pub(crate) fn start_image(&mut self, dest_url: &str) {
        self.in_image = true;
        self.image_url = dest_url.to_string();
        self.image_alt.clear();
    }

    pub(crate) fn end_image(&mut self) -> io::Result<()> {
        let image_url = self.image_url.clone();
        let image_alt = self.image_alt.clone();
        self.render_image(&image_url, &image_alt)?;
        self.in_image = false;
        self.image_url.clear();
        self.image_alt.clear();
        self.last_was_space = false;
        Ok(())
    }

    pub(crate) fn start_code_block(
        &mut self,
        kind: pulldown_cmark::CodeBlockKind,
    ) -> io::Result<()> {
        self.in_code_block = true;
        self.code_buf.clear();
        self.code_lang = match kind {
            pulldown_cmark::CodeBlockKind::Fenced(info) => {
                let s = info.split_whitespace().next().unwrap_or("").to_string();
                if s.is_empty() { None } else { Some(s) }
            }
            pulldown_cmark::CodeBlockKind::Indented => None,
        };
        self.ensure_newline()?;
        Ok(())
    }

    pub(crate) fn end_code_block(&mut self) -> io::Result<()> {
        let code = std::mem::take(&mut self.code_buf);
        let lang_opt = self.code_lang.take();
        self.render_code_block(&code, lang_opt.as_deref())?;
        self.in_code_block = false;
        Ok(())
    }

    pub(crate) fn start_blockquote(&mut self) -> io::Result<()> {
        self.in_blockquote = true;
        self.render_blockquote_start()
    }

    pub(crate) fn end_blockquote(&mut self) {
        self.in_blockquote = false;
    }

    pub(crate) fn start_table(&mut self) -> io::Result<()> {
        self.in_table = true;
        self.in_table_head = false;
        self.ensure_newline()?;
        Ok(())
    }

    pub(crate) fn start_table_head(&mut self) {
        self.in_table_head = true;
        self.current_row.clear();
    }

    pub(crate) fn start_table_row(&mut self) {
        if !self.in_table_head {
            self.current_row.clear();
        }
    }

    pub(crate) fn start_table_cell(&mut self) {
        self.current_cell.clear();
    }

    pub(crate) fn end_table_cell(&mut self) {
        self.current_row.push(self.current_cell.clone());
        self.current_cell.clear();
    }

    pub(crate) fn end_table_row(&mut self) {
        self.table_rows.push(self.current_row.clone());
        self.current_row.clear();
        self.last_was_space = true;
    }

    pub(crate) fn end_table_head(&mut self) {
        if !self.current_row.is_empty() {
            self.table_rows.push(self.current_row.clone());
            self.current_row.clear();
        }
        self.table_header_count = self.table_rows.len();
        self.in_table_head = false;
    }

    pub(crate) fn end_table(&mut self) -> io::Result<()> {
        self.render_table()?;
        self.table_rows.clear();
        self.table_header_count = 0;
        self.in_table = false;
        writeln!(self.output)?;
        self.last_was_space = true;
        Ok(())
    }

    pub(crate) fn write_event_text(&mut self, text: &str) -> io::Result<()> {
        if self.in_code_block {
            self.code_buf.push_str(text);
            return Ok(());
        }
        if self.in_table {
            self.current_cell.push_str(text);
            return Ok(());
        }
        self.write_text(text)
    }

    pub(crate) fn write_event_code(&mut self, code: &str) -> io::Result<()> {
        if self.in_table {
            self.current_cell.push('`');
            self.current_cell.push_str(code);
            self.current_cell.push('`');
            return Ok(());
        }
        if !self.in_code_block {
            self.render_inline_code(code)?;
        }
        Ok(())
    }

    pub(crate) fn soft_break(&mut self) -> io::Result<()> {
        if self.heading_level.is_some() {
            self.push_heading_softbreak();
            Ok(())
        } else if self.in_blockquote {
            self.render_blockquote_start()
        } else {
            write!(self.output, " ")?;
            self.last_was_space = true;
            Ok(())
        }
    }

    pub(crate) fn hard_break(&mut self) -> io::Result<()> {
        writeln!(self.output)?;
        self.last_was_space = true;
        Ok(())
    }

    pub(crate) fn end_paragraph(&mut self) -> io::Result<()> {
        if self.list_depth == 0 {
            writeln!(self.output)?;
            writeln!(self.output)?;
        } else {
            writeln!(self.output)?;
        }
        self.last_was_space = true;
        Ok(())
    }

    // --- existing rendering routines below (unchanged) ---
    pub(crate) fn render_heading(&mut self, level: u32, text: &str) -> io::Result<()> {
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

    pub(crate) fn write_text(&mut self, text: &str) -> io::Result<()> {
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

    pub(crate) fn render_list_item_start(&mut self) -> io::Result<()> {
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

    pub(crate) fn render_inline_code(&mut self, code: &str) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red));
        self.output.set_color(&spec)?;
        write!(self.output, "`{}`", code)?;
        self.output.reset()?;
        self.last_was_space = false;
        Ok(())
    }

    pub(crate) fn render_image(&mut self, url: &str, alt: &str) -> io::Result<()> {
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

    pub(crate) fn render_blockquote_start(&mut self) -> io::Result<()> {
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

    pub(crate) fn render_rule(&mut self) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::White)).set_intense(false);
        self.output.set_color(&spec)?;
        writeln!(self.output, "{}", "─".repeat(60))?;
        self.output.reset()?;
        self.last_was_space = true;
        Ok(())
    }

    pub(crate) fn render_task_list_item(&mut self, checked: bool) -> io::Result<()> {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));
        let checkbox = if checked { "☑" } else { "☐" };
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(if checked { Color::Green } else { Color::White }));
        self.output.set_color(&spec)?;
        write!(self.output, "{}{} ", indent, checkbox)?;
        self.output.reset()?;
        Ok(())
    }

    pub(crate) fn render_code_block(&mut self, code: &str, lang: Option<&str>) -> io::Result<()> {
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

    pub(crate) fn render_table(&mut self) -> io::Result<()> {
        table::render_table(self)
    }
}
