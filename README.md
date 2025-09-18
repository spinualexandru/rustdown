# rustdown — terminal Markdown renderer

A minimal, colorized Markdown renderer for the terminal using [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) and termcolor.

## Demo
[Demo](https://github.com/user-attachments/assets/dc35588b-6040-4c76-89b6-86521f867754)


## Quick start

- Build: cargo build
- Run (renders embedded reference): cargo run
- Render a file: cargo run -- PATH/TO/FILE.md

# Usage examples

- Render the built-in reference:

  cargo run

- Render a specific file:

  cargo run -- references/full.md

- Page colored output through less (preserve colors):

  cargo run -- references/full.md | less -R

- Run in release mode for faster startup:

  cargo run --release -- docs/long.md

## Features

- Headers (H1..H6) with color and ASCII underlines
- Emphasis, bold, strikethrough
- Ordered + unordered lists with nesting
- Task lists (checked/unchecked)
- Links and images (displayed as text with URL)
- Code blocks and inline code
- Blockquotes, horizontal rules, footnotes

## Requirements

- Rust 1.56+ (edition 2021)

## License
WTFPL
