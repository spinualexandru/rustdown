pub mod markdown;
pub mod render;

/// Render markdown to any `termcolor::WriteColor` output.
///
/// This is the same renderer used by the CLI, but exposes a writer-injected entrypoint
/// for testing and embedding.
pub fn render_markdown_to_writer(
    md: &str,
    preserve_fences: bool,
    output: &mut impl termcolor::WriteColor,
) -> std::io::Result<()> {
    markdown::render_markdown(md, preserve_fences, output)
}
