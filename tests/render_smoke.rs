use rustdown::render_markdown_to_writer;
use std::io;
use termcolor::{ColorSpec, WriteColor};

#[derive(Default)]
struct TestWriter {
    buf: Vec<u8>,
}

impl io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl WriteColor for TestWriter {
    fn supports_color(&self) -> bool {
        false
    }

    fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn as_string(w: &TestWriter) -> String {
    String::from_utf8(w.buf.clone()).expect("renderer output must be valid utf-8")
}

#[test]
fn heading_renders_with_underline() {
    let md = "# Title\n\nParagraph\n";
    let mut w = TestWriter::default();
    render_markdown_to_writer(md, false, &mut w).unwrap();

    let s = as_string(&w);
    assert!(s.contains("Title\n"));
    assert!(s.contains("=====\n"));
}

#[test]
fn preserve_fences_keeps_backticks() {
    let md = "```rust\nfn main() {}\n```\n";

    let mut w = TestWriter::default();
    render_markdown_to_writer(md, true, &mut w).unwrap();

    let s = as_string(&w);
    assert!(s.contains("```rust\n"));
    assert!(s.contains("fn main() {}\n"));
    assert!(s.contains("```\n"));
}

use rustdown::{LinkStyle, RenderOptions, Theme, render_markdown_with_options};
use unicode_width::UnicodeWidthStr;

fn render(md: &str) -> String {
    render_options(md, RenderOptions::default())
}
fn render_options(md: &str, options: RenderOptions) -> String {
    let mut writer = TestWriter::default();
    render_markdown_with_options(md, &options, &mut writer).unwrap();
    as_string(&writer)
}

#[test]
fn heading_keeps_code_links_images_and_notes_in_order() {
    let s = render("# Install `rustdown` with [Cargo](https://example.com)\n");
    assert_eq!(
        s.lines().next(),
        Some("Install `rustdown` with Cargo (https://example.com)")
    );
    let s = render("# A ![logo](logo.png) and note[^a]\n\n[^a]: Explained.\n");
    assert!(s.starts_with("A [image: logo] (logo.png) and note[^a]\n"));
    assert!(s.contains("[^a]: Explained."));
}

#[test]
fn table_keeps_rich_content_inside_cells() {
    let s = render(
        "| Link | Image | Note |\n| --- | --- | --- |\n| [docs](https://example.com) | ![logo](logo.png) | ref[^a] |\n\n[^a]: A note.\n",
    );
    assert!(s.starts_with('┌'));
    let row = s.lines().find(|l| l.contains("docs")).unwrap();
    assert!(row.contains("docs (https://example.com)"));
    assert!(row.contains("[image: logo] (logo.png)"));
    assert!(row.contains("ref[^a]"));
    assert!(row.ends_with('│'));
    assert!(s.contains("[^a]: A note."));
}

#[test]
fn table_alignment_and_unicode_borders_use_terminal_columns() {
    let s = render(
        "| Name | Center | Amount |\n| :--- | :---: | ---: |\n| a | x | 1 |\n| 日本語 | yyy | 12345 |\n| e\u{301} | 👩‍💻 | 5 |\n",
    );
    let widths: Vec<_> = s.lines().map(UnicodeWidthStr::width).collect();
    assert!(widths.iter().all(|w| *w == widths[0]), "{s}");
    assert!(s.contains("│ a      │   x    │      1 │"), "{s}");
}

#[test]
fn headings_measure_display_width() {
    assert_eq!(render("# 日本語\n"), "日本語\n======\n");
    assert_eq!(render("# e\u{301}\n"), "e\u{301}\n=\n");
}

#[test]
fn ordered_lists_respect_starts_and_restart_sibling_children() {
    assert_eq!(render("7. Seven\n8. Eight\n"), "7. Seven\n8. Eight\n");
    assert_eq!(
        render("1. First\n   1. Child A\n   2. Child B\n2. Second\n   1. Child C\n   2. Child D\n"),
        "1. First\n   1. Child A\n   2. Child B\n2. Second\n   1. Child C\n   2. Child D\n"
    );
}

#[test]
fn mixed_list_nesting_and_multi_digit_markers_align() {
    let s = render_options(
        "9. Nine\n10. Ten with a longer continuation that should wrap\n    - Nested\n",
        RenderOptions {
            width: 22,
            ..Default::default()
        },
    );
    assert!(
        s.contains("10. Ten with a longer\n    continuation that\n    should wrap\n    • Nested\n"),
        "{s}"
    );
}

#[test]
fn list_continuations_and_child_blocks_keep_indentation() {
    let s = render(
        "- First  \n  continuation\n\n  Second paragraph.\n\n  > A quote\n\n  ```rust\n  let x = 1;\n  ```\n\n- Next\n",
    );
    assert!(s.starts_with("• First\n  continuation\n"));
    assert!(s.contains("  Second paragraph.\n"));
    assert!(s.contains("  │ A quote\n"));
    assert!(s.contains("  │ let x = 1;\n"));
    assert!(s.ends_with("\n• Next\n"));
}

#[test]
fn task_markers_are_not_duplicated_or_double_indented() {
    assert_eq!(
        render("- [x] Done\n  - [ ] Child\n- [ ] Next\n"),
        "☑ Done\n  ☐ Child\n☐ Next\n"
    );
    assert_eq!(
        render("7. [x] Done\n8. [ ] Next\n"),
        "7. ☑ Done\n8. ☐ Next\n"
    );
}

#[test]
fn quotes_keep_rails_across_nested_blocks_and_hard_breaks() {
    let s = render(
        "> First  \n> continuation\n>\n> Second\n>\n> > Inner\n>\n> Outer again\n>\n> - Item\n>\n> ```\n> code\n> ```\n",
    );
    assert!(s.lines().all(|l| l.starts_with("│ ")), "{s}");
    assert!(s.contains("│ │ Inner\n"));
    assert!(s.contains("│ Outer again\n"));
    assert!(s.contains("│ • Item\n"));
    assert!(s.contains("│ │ code\n"));
}

#[test]
fn prose_tables_and_rules_fit_40_80_and_120_columns() {
    let words = "Some long content with **bold** and `code` repeated here. ".repeat(8);
    let md = format!(
        "- {words}\n\n> {words}\n\n| Description | Status |\n| --- | --- |\n| {words} | OK |\n\n---\n"
    );
    for width in [40, 80, 120] {
        let s = render_options(
            &md,
            RenderOptions {
                width,
                ..Default::default()
            },
        );
        for line in s.lines() {
            assert!(line.width() <= width, "width {width}: {line}");
        }
        assert_eq!(s.lines().last().unwrap().width(), width);
        assert!(
            s.contains("Status"),
            "short columns should not wrap unnecessarily: {s}"
        );
    }
}

#[test]
fn narrow_table_falls_back_to_labeled_records() {
    let s = render_options(
        "| Name | State | Count |\n| --- | --- | --- |\n| Long name | Ready | 12 |\n",
        RenderOptions {
            width: 16,
            ..Default::default()
        },
    );
    assert_eq!(s, "Name\n  Long name\nState\n  Ready\nCount\n  12\n");
}

#[test]
fn long_words_wrap_without_splitting_graphemes() {
    let s = render_options(
        "👩‍💻👩‍💻👩‍💻 e\u{301}e\u{301}e\u{301}\n",
        RenderOptions {
            width: 4,
            ..Default::default()
        },
    );
    assert_eq!(s, "👩‍💻👩‍💻\n👩‍💻\ne\u{301}e\u{301}e\u{301}\n");
}

#[test]
fn footnotes_are_labeled_and_keep_multiple_paragraphs() {
    let s = render("Text[^alpha].\n\n[^alpha]: First **note**.\n\n    Second paragraph.\n");
    assert!(s.contains("Text[^alpha].\n\nNotes\n\n[^alpha]: First note.\n"));
    assert!(s.contains("          Second paragraph.\n"), "{s}");
}

#[test]
fn mixed_html_keeps_markdown_and_breaks() {
    let s = render(
        "<div>Intro <b>bold</b></div>\n\n# Heading\n\nBefore<br>after and <kbd>Enter</kbd>.\n",
    );
    assert_eq!(
        s,
        "Intro bold\n\nHeading\n=======\n\nBefore\nafter and Enter.\n"
    );
    assert!(render("Text <custom>inside</custom>\n").contains("<custom>inside</custom>"));
}

#[test]
fn links_avoid_repeating_destinations_and_can_hide_them() {
    assert_eq!(render("<https://example.com>\n"), "https://example.com\n");
    assert_eq!(render("<alex@example.com>\n"), "alex@example.com\n");
    assert_eq!(
        render_options(
            "[docs](https://example.com) ![logo](logo.png)\n",
            RenderOptions {
                links: LinkStyle::Text,
                ..Default::default()
            }
        ),
        "docs [image: logo]\n"
    );
}

#[test]
fn alerts_have_labels_and_preserve_nested_quote_structure() {
    assert_eq!(
        render("> [!WARNING]\n> Save your work.\n"),
        "│ WARNING\n│ Save your work.\n"
    );
}

#[test]
fn preserve_fences_keeps_delimiters_info_and_embedded_shorter_fences() {
    let md = "~~~~rust title=demo\n```\nlet x = 1;\n~~~~~\n";
    assert_eq!(
        render_options(
            md,
            RenderOptions {
                preserve_fences: true,
                ..Default::default()
            }
        ),
        md
    );
    let nested = render_options(
        "> ````rust title=demo\n> ```\n> ````\n",
        RenderOptions {
            preserve_fences: true,
            ..Default::default()
        },
    );
    assert_eq!(nested, "│ ````rust title=demo\n│ ```\n│ ````\n");
}

#[test]
fn unclosed_fences_do_not_gain_a_synthetic_closing_delimiter() {
    assert_eq!(
        render_options(
            "~~~rust\nlet x = 1;\n",
            RenderOptions {
                preserve_fences: true,
                ..Default::default()
            }
        ),
        "~~~rust\nlet x = 1;\n"
    );
}

#[test]
fn code_preserves_whitespace_and_does_not_wrap() {
    let s = render_options(
        "```unknown\n\tindented  \n\n01234567890123456789012345\n```\n",
        RenderOptions {
            width: 20,
            syntax_highlighting: true,
            ..Default::default()
        },
    );
    assert!(
        s.contains("│ \tindented  \n│ \n│ 01234567890123456789012345\n"),
        "{s}"
    );
}

#[test]
fn terminal_controls_are_removed_from_all_content_paths() {
    let md = "# A\x1b[2JB\n\n```rust\na\x1b[2Jb\x1b]0;title\x07c\u{009b}31md\x08e\n```\n\n[x](https://example.com/\x1b[31mred)\n\n<div>a\x1bPpayload\x1b\\b</div>\n";
    let s = render(md);
    assert!(!s.chars().any(|c| c.is_control() && c != '\n' && c != '\t'));
    assert!(s.starts_with("AB\n"));
    assert!(s.contains("abcde"));
    assert!(s.contains("https://example.com/red"));
    assert!(s.ends_with("ab\n"));
}

#[test]
fn ascii_mode_changes_decorations_without_changing_content() {
    let s = render_options(
        "- [x] 日本語\n\n> Quote\n\n| A | B |\n| --- | --- |\n| x | y |\n\n---\n",
        RenderOptions {
            ascii: true,
            width: 30,
            ..Default::default()
        },
    );
    assert!(s.starts_with("[x] 日本語\n"));
    assert!(s.contains("> Quote\n"));
    assert!(!s.contains(['│', '─', '☑', '•']));
    assert!(s.contains("+-----+-----+"));
}

#[derive(Default)]
struct StyleWriter {
    entries: Vec<(String, ColorSpec)>,
    current: ColorSpec,
}
impl io::Write for StyleWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.entries.push((
            String::from_utf8(bytes.to_vec()).unwrap(),
            self.current.clone(),
        ));
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl WriteColor for StyleWriter {
    fn supports_color(&self) -> bool {
        true
    }
    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        self.current = spec.clone();
        Ok(())
    }
    fn reset(&mut self) -> io::Result<()> {
        self.current = ColorSpec::new();
        Ok(())
    }
}

#[test]
fn nested_styles_and_inline_code_keep_outer_styles() {
    let mut w = StyleWriter::default();
    render_markdown_to_writer(
        "*outer **bold *inner* bold** outer again* and **bold `code`** and ~~`gone`~~\n",
        false,
        &mut w,
    )
    .unwrap();
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("outer again") && c.italic())
    );
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("`code`") && c.bold())
    );
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("`gone`") && c.strikethrough())
    );
}

#[test]
fn tables_preserve_styles_and_emphasize_headers() {
    let mut w = StyleWriter::default();
    render_markdown_to_writer(
        "| Header |\n| --- |\n| **Bold** and *Italic* |\n",
        false,
        &mut w,
    )
    .unwrap();
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("Header") && c.bold())
    );
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("Bold") && c.bold())
    );
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("Italic") && c.italic())
    );
}

#[test]
fn highlighting_is_optional_and_supports_light_and_dark_palettes() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut w = StyleWriter::default();
        render_markdown_with_options(
            "```rust\nfn main() { let n = 42; }\n```\n",
            &RenderOptions {
                syntax_highlighting: true,
                theme,
                ..Default::default()
            },
            &mut w,
        )
        .unwrap();
        assert!(
            w.entries
                .iter()
                .any(|(_, s)| matches!(s.fg(), Some(termcolor::Color::Rgb(..))))
        );
    }
    let mut w = StyleWriter::default();
    render_markdown_to_writer("```rust\nfn main() {}\n```\n", false, &mut w).unwrap();
    assert!(
        w.entries
            .iter()
            .any(|(s, style)| s.contains("fn main()") && style.fg().is_none())
    );
}

#[test]
fn invalid_width_is_an_error_before_writing() {
    for width in [0, 10_001] {
        let mut w = TestWriter::default();
        let err = render_markdown_with_options(
            "text",
            &RenderOptions {
                width,
                ..Default::default()
            },
            &mut w,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(w.buf.is_empty());
    }
}

#[test]
fn output_errors_propagate() {
    struct Broken;
    impl io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl WriteColor for Broken {
        fn supports_color(&self) -> bool {
            false
        }
        fn set_color(&mut self, _: &ColorSpec) -> io::Result<()> {
            Ok(())
        }
        fn reset(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        render_markdown_to_writer("text", false, &mut Broken)
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn decoded_entities_cannot_reintroduce_terminal_controls() {
    let s = render(
        "before&#27;[2Jafter and &#155;31mred\n\n# Heading &#27;[0m\n\n| Value |\n| --- |\n| &#27;[2Jtext |\n",
    );
    assert!(
        !s.chars().any(|c| c.is_control() && c != '\n' && c != '\t'),
        "{s:?}"
    );
    assert!(s.contains("after"));
    assert!(s.contains("text"));
}

#[test]
fn reset_color_and_flush_errors_propagate() {
    struct FailingWriter(&'static str);
    impl io::Write for FailingWriter {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            Ok(data.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.0 == "flush" {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }
    impl WriteColor for FailingWriter {
        fn supports_color(&self) -> bool {
            true
        }
        fn set_color(&mut self, _: &ColorSpec) -> io::Result<()> {
            if self.0 == "color" {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
        fn reset(&mut self) -> io::Result<()> {
            if self.0 == "reset" {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }
    for operation in ["color", "reset", "flush"] {
        let error = render_markdown_to_writer("# Heading", false, &mut FailingWriter(operation))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }
}

#[test]
fn closing_fences_accept_tabs_and_preserve_original_delimiters() {
    for marker in ["```", "~~~~"] {
        let md = format!("{marker}rust title=test\nlet x = 1;\n{marker}\t \t\n\n# After\n");
        let s = render_options(
            &md,
            RenderOptions {
                preserve_fences: true,
                ..Default::default()
            },
        );
        assert!(
            s.contains(&format!("{marker}\t \t\n\nAfter\n=====")),
            "{s:?}"
        );
    }
    let s = render_options(
        "> ```\n> code\n> ```\t\n>\n> After\n",
        RenderOptions {
            preserve_fences: true,
            ..Default::default()
        },
    );
    assert_eq!(s, "│ ```\n│ code\n│ ```\t\n│ \n│ After\n");
    let s = render("- ```\n  code\n  ```\t\n\n  After\n");
    assert!(s.contains("  After"), "{s}");
    assert!(!s.contains("│ ```"), "{s}");
}

#[test]
fn fence_workaround_does_not_change_code_tabs_or_html() {
    let md = "````\n```\t\n> ~~~\t\n\t\tindented\n````\t\n";
    assert_eq!(
        render_options(
            md,
            RenderOptions {
                preserve_fences: true,
                ..Default::default()
            }
        ),
        md
    );
    let s = render_options("    ```\t\n    content\t\n", RenderOptions::default());
    assert!(s.contains("│ ```\t\n│ content\t\n"), "{s:?}");
    let s = render_options(
        "> ````\n> ```\t\n> ````\t\n",
        RenderOptions {
            preserve_fences: true,
            ..Default::default()
        },
    );
    assert_eq!(s, "│ ````\n│ ```\t\n│ ````\t\n");
    let s = render("```` inline\n```\t\nend ````\n");
    assert!(s.contains("```\t"), "{s:?}");
}

#[test]
fn every_non_preserved_block_fits_even_at_one_column() {
    let md = "# 日本語\n\n> > > > A 👩‍💻 quote\n\n123456789. [x] A long item\n           - child\n\nText[^long_footnote_name].\n\n[^long_footnote_name]: Notes with 日本語.\n\n| 名 | Other |\n| --- | --- |\n| 👩‍💻 | abc |\n\n```rust\n\tlet x = 123456789;\n```\n\n[Long label](https://example.com/long/path)\n\n---\n";
    for columns in 1..=30 {
        let s = render_options(
            md,
            RenderOptions {
                width: columns,
                wrap_code: true,
                links: LinkStyle::Reference,
                ..Default::default()
            },
        );
        for line in s.lines() {
            assert!(
                line.width() <= columns,
                "width {columns}, {}: {line:?}",
                line.width()
            );
        }
    }
}

#[test]
fn oversized_labels_move_above_their_content() {
    let s = render_options(
        "Text[^averylongname].\n\n[^averylongname]: Content here.\n",
        RenderOptions {
            width: 10,
            ..Default::default()
        },
    );
    assert!(s.contains("Content\nhere.\n"), "{s}");
    assert!(s.lines().all(|line| line.width() <= 10));
    let s = render_options(
        "123456789. Item\n",
        RenderOptions {
            width: 8,
            ..Default::default()
        },
    );
    assert!(s.ends_with("Item\n"));
    assert!(s.lines().all(|line| line.width() <= 8));
}

#[test]
fn code_wrapping_retains_all_spaces_and_expands_tabs() {
    let s = render_options(
        "```\n\tabc  def    xyz  \n\n```\n",
        RenderOptions {
            width: 10,
            wrap_code: true,
            ..Default::default()
        },
    );
    assert!(s.lines().all(|l| l.width() <= 10), "{s}");
    let body: String = s
        .lines()
        .filter_map(|line| line.strip_prefix("│ "))
        .collect();
    assert_eq!(body, "    abc  def    xyz  ");
    assert!(s.contains("│ \n"));
}

#[test]
fn inline_html_links_images_and_typography_compose_with_markdown() {
    let s = render(
        "Use <a href='https://example.com?q=1&amp;x=2'>**docs**</a>, <img alt='A &amp; B' src='logo.png'> and H<sub>2</sub>O or x<sup>n + 1</sup>.\n",
    );
    assert!(s.contains("docs (https://example.com?q=1&x=2)"), "{s}");
    assert!(s.contains("[image: A & B] (logo.png)"), "{s}");
    assert!(s.contains("H_(2)O"));
    assert!(s.contains("x^(n + 1)"));
    assert_eq!(
        render("<span title='a>b'>Text <small>small</small></span><!-- hidden -->\n"),
        "Text small\n"
    );
    let mut w = StyleWriter::default();
    render_markdown_to_writer(
        "**Outer <code>code</code> bold** <mark>mark</mark> <ins>added</ins>\n",
        false,
        &mut w,
    )
    .unwrap();
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("code") && c.bold())
    );
    assert!(
        w.entries
            .iter()
            .any(|(s, c)| s.contains("added") && c.underline())
    );
}

#[test]
fn unclosed_html_anchors_and_adjacent_links_keep_separate_labels() {
    assert_eq!(
        render("<a href='https://a.test'>A</a><a href='https://b.test'>B</a>\n"),
        "A (https://a.test)B (https://b.test)\n"
    );
    assert_eq!(
        render("Text <a href='https://a.test'>unclosed **bold**\n"),
        "Text unclosed bold (https://a.test)\n"
    );
}

#[test]
fn references_deduplicate_markdown_html_and_image_destinations() {
    let s = render_options(
        "[one](https://example.com) <a href='https://example.com'>two</a> ![logo](logo.png)\n\n<div><a href='https://example.com'>three</a></div>\n\nNote[^a].\n\n[^a]: [last](https://other.test)\n",
        RenderOptions {
            links: LinkStyle::Reference,
            ..Default::default()
        },
    );
    assert!(s.contains("one[1] two[1] [image: logo][2]"), "{s}");
    assert!(s.contains("three[1]"));
    assert!(s.contains("last[3]"));
    assert!(
        s.ends_with("Links\n\n[1]: https://example.com\n[2]: logo.png\n[3]: https://other.test\n"),
        "{s}"
    );
}

fn ansi_options(md: &str, options: RenderOptions) -> String {
    let mut writer = termcolor::Buffer::ansi();
    render_markdown_with_options(md, &options, &mut writer).unwrap();
    String::from_utf8(writer.as_slice().to_vec()).unwrap()
}

#[test]
fn hyperlinks_survive_wrapping_and_close_before_newlines_and_padding() {
    let s = ansi_options(
        "[a long label with **bold** words](https://example.com)\n\n| Link |\n| --- |\n| [cell](https://example.com) |\n",
        RenderOptions {
            width: 12,
            links: LinkStyle::Hyperlink,
            ..Default::default()
        },
    );
    let open = "\x1b]8;;https://example.com\x1b\\";
    let close = "\x1b]8;;\x1b\\";
    assert!(s.matches(open).count() >= 4, "{s:?}");
    assert_eq!(s.matches(open).count(), s.matches(close).count());
    for line in s.lines() {
        assert_eq!(line.matches(open).count(), line.matches(close).count());
    }
    assert!(!s.contains("(https://example.com)"));
    assert!(s.contains(&format!("cell{close}")));
}

#[test]
fn hyperlink_mode_falls_back_for_plain_writers_and_unsafe_schemes() {
    let options = RenderOptions {
        links: LinkStyle::Hyperlink,
        ..Default::default()
    };
    assert_eq!(
        render_options("[docs](https://example.com)\n", options.clone()),
        "docs (https://example.com)\n"
    );
    let s = ansi_options("[run](javascript:alert) [path](relative.md)\n", options);
    assert!(!s.contains("\x1b]8;"));
    assert!(s.contains("javascript:alert"));
    assert!(s.contains("relative.md"));
}

#[test]
fn decoded_html_attributes_cannot_inject_hyperlink_commands() {
    let s = ansi_options(
        "Click <a href='https://example.com/&#27;]0;title&#7;'>label</a>\n",
        RenderOptions {
            links: LinkStyle::Hyperlink,
            ..Default::default()
        },
    );
    assert!(!s.contains("\x1b]0;"), "{s:?}");
    assert!(!s.contains('\x07'));
    assert_eq!(s.matches("\x1b]8;").count(), 2);
}

#[test]
fn long_footnote_labels_leave_a_readable_body_width() {
    let s = render_options(
        "Text[^a_very_long_footnote_identifier].\n\n[^a_very_long_footnote_identifier]: Labels can move above the content.\n",
        RenderOptions {
            width: 40,
            ..Default::default()
        },
    );
    assert!(
        s.contains("[^a_very_long_footnote_identifier]:\nLabels can move above the content.\n"),
        "{s}"
    );
}

#[test]
fn hyperlink_errors_propagate_and_failed_labels_close_the_link() {
    struct Writer {
        calls: usize,
        fail_link_at: usize,
        fail_text: bool,
    }
    impl io::Write for Writer {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            if self.fail_text {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(data.len())
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl WriteColor for Writer {
        fn supports_color(&self) -> bool {
            true
        }
        fn supports_hyperlinks(&self) -> bool {
            true
        }
        fn set_color(&mut self, _: &ColorSpec) -> io::Result<()> {
            Ok(())
        }
        fn reset(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn set_hyperlink(&mut self, _: &termcolor::HyperlinkSpec<'_>) -> io::Result<()> {
            self.calls += 1;
            if self.calls == self.fail_link_at {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }
    for (fail_link_at, fail_text, expected_calls) in [(1, false, 1), (2, false, 2), (0, true, 2)] {
        let mut writer = Writer {
            calls: 0,
            fail_link_at,
            fail_text,
        };
        let err = render_markdown_with_options(
            "[docs](https://example.com)",
            &RenderOptions {
                links: LinkStyle::Hyperlink,
                ..Default::default()
            },
            &mut writer,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(writer.calls, expected_calls);
    }
}
