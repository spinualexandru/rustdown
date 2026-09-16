# rustdown

Render Markdown in your terminal, with readable tables, nested lists and quotes, and optional syntax highlighting. Also available as a writer-injected Rust library.

## Install

```sh
cargo install rustdown-cli
```

## Use

```sh
rustdown README.md
printf '# Hello\n\nSome **Markdown**.\n' | rustdown
rustdown '# An inline heading'
rustdown html '<h1>Hello</h1><p>HTML content</p>'
```

Without a positional argument, rustdown reads stdin. An interactive invocation without input reports a usage error.

```sh
rustdown --width 80 README.md
rustdown --highlight --wrap-code references/test.md
rustdown --theme light --highlight README.md
rustdown --preserve-fences README.md
rustdown --links text --ascii README.md
rustdown --links reference README.md
rustdown --links hyperlink README.md
rustdown --color always README.md | less -R
```

| Option | Behavior |
| --- | --- |
| `--width N` | Available columns, from 1 to 10000. Defaults to terminal width, or 80 when redirected. |
| `--color auto\|always\|never` | Auto disables colors when redirected or when `NO_COLOR` is set. An explicit choice takes precedence. |
| `--theme dark\|light` | Background palette for accents and syntax highlighting. Defaults to dark. |
| `--highlight` | Highlight recognized fenced-code languages. Unknown languages remain plain. |
| `--wrap-code` | Wrap long code lines without dropping spaces; expand tabs at four-column stops. Off by default. |
| `--preserve-fences` | Keep the original fence character, length, and info string. Also enabled by `RUSTDOWN_PRESERVE_FENCES`. |
| `--links inline\|text\|hyperlink\|reference` | Show inline URLs, labels only, clickable terminal links, or numbered references with a Links section. |
| `--ascii` | Use ASCII borders, bullets, and checkboxes. Document text is unchanged. |

Options work before or after the `html` subcommand. `html` reads a string, file, or stdin using the same input rules as Markdown.

## Rendering

- Headings, emphasis, bold, strikethrough, inline code, links, and image descriptions retain their structure when combined.
- Prose wraps to the available width. Lists use hanging indentation and preserve starting numbers; quotes retain their rails across paragraphs and child blocks. On extremely narrow layouts, markers and long labels move onto separate lines so content still fits.
- Tables preserve inline formatting and column alignment. Cells wrap; tables with too many columns for the available space become labeled records.
- Footnotes appear in a labeled notes section. GitHub note, tip, important, warning, and caution alerts have distinct labels.
- Width calculations account for CJK text, combining accents, and joined emoji. A glyph wider than the available space is displayed as Unicode escapes, preserving its value without splitting it. Emoji display can still vary by terminal and font.
- Code blocks preserve whitespace and remain unwrapped by default. Use `--wrap-code` to fit code to the available width; spaces are retained and tabs expand at four-column stops. `--preserve-fences` always keeps original delimiters and info strings, which may exceed the selected width even when the code body wraps.
- Mixed HTML blocks are converted locally; Markdown outside those blocks stays Markdown. Inline links, images, line breaks, emphasis, strikethrough, underline, code, keyboard text, and common phrasing tags are supported. Superscripts/subscripts use `^(text)`/`_(text)` notation. Attribute parsing supports quoted values and HTML entities. Comments are hidden; unsupported tags remain visible. Use `rustdown html` to convert an entire HTML document explicitly.
- Input terminal control sequences are removed from all content, including code and URLs. In hyperlink mode, rustdown generates OSC 8 links for HTTP, HTTPS, mail, and file URLs; other destinations stay visible as text. Hyperlinks fall back to inline URLs for writers without hyperlink support (including automatic redirected output). Use `--color always` to retain hyperlinks when piping to a compatible pager. No destinations are fetched.
- Reference mode deduplicates destinations across Markdown, HTML, images, and footnotes, then lists them at the end. Autolinks in inline mode never repeat their URL.

Closing fences with trailing tabs are supported through a parser adapter that preserves original code whitespace and delimiters. No true terminal image rendering or math typesetting is performed. HTML conversion follows `html2md` and does not execute scripts.

## Library

Add the package under the `rustdown` dependency name:

```toml
[dependencies]
rustdown = { package = "rustdown-cli", version = "0.1.3" }
termcolor = "1.4.1"
```

```rust
use rustdown::{RenderOptions, render_markdown_with_options};
use termcolor::Buffer;

let mut output = Buffer::no_color();
let options = RenderOptions {
    width: 60,
    ..RenderOptions::default()
};
render_markdown_with_options("# Hello\n\nSome **Markdown**.", &options, &mut output).unwrap();
```

Set `RenderOptions::wrap_code` and `LinkStyle::Hyperlink` or `LinkStyle::Reference` for the corresponding library features.

The existing `render_markdown_to_writer(markdown, preserve_fences, writer)` entry point remains available with an 80-column default. Library rendering uses explicit options and the writer's color policy; it does not inspect the terminal or environment. Write, color, reset, and flush failures propagate as I/O errors.

## Try the samples

```sh
cargo run -- references/rendering-audit.md
cargo run -- --width 40 references/rendering-audit.md
cargo run -- --highlight --theme light references/test.md
cargo test
```

For an optional check against Kitty's native terminal parser, install Kitty, run `cargo build`, then:

```sh
kitty +runpy 'import runpy; runpy.run_path("scripts/check-kitty.py", run_name="__main__")'
```

This validates terminal cells and hyperlink metadata at widths from 1 to 120 columns without opening a window. It does not verify native font rasterization.
