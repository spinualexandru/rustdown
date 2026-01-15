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
