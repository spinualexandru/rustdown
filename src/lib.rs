pub mod markdown;
pub mod render;

pub use render::{LinkStyle, RenderOptions, Theme};

/// Render Markdown using an 80-column layout and a writer chosen by the caller.
/// Use [`render_markdown_with_options`] for explicit width and presentation controls.
pub fn render_markdown_to_writer(
    md: &str,
    preserve_fences: bool,
    output: &mut impl termcolor::WriteColor,
) -> std::io::Result<()> {
    markdown::render_markdown(md, preserve_fences, output)
}

/// Render Markdown with explicit options. The writer determines whether colors are emitted.
/// Code stays unwrapped unless `wrap_code` is enabled. Preserved fences keep their original
/// delimiters even if they exceed the width. Hyperlinks fall back to inline URLs when unsupported.
pub fn render_markdown_with_options(
    md: &str,
    options: &RenderOptions,
    output: &mut impl termcolor::WriteColor,
) -> std::io::Result<()> {
    markdown::render_with_options(md, options, output)
}
