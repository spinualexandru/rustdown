# Rendering audit

Investigated locally on 2026-09-16 against rustdown-cli 0.1.3. Findings below describe the original renderer.

## Implementation follow-up

The working tree now fixes the structural defects and adds width-aware layout, Unicode measurement, dark/light palettes, optional syntax highlighting, alerts, link controls, and original-fence preservation. The existing writer API remains available; `RenderOptions` exposes the new controls.

The follow-up adds `--wrap-code`, clickable and referenced link modes, more complete inline HTML, trailing-tab fence support, and narrow-layout fallbacks down to one column. Native Kitty parser validation checks terminal cells and hyperlink metadata; native font rasterization and Ghostty windows have not been inspected. See `scripts/check-kitty.py` for the repeatable headless check.

Current validation: 51 Rust tests pass (43 renderer integration tests, 7 CLI tests, and 1 parser regression). Formatting and strict Clippy checks pass. Kitty 0.48.2's native parser passes 18 cases covering widths 1/8/20/40/80/120 and inline/reference/hyperlink modes, checking 12,827 output lines. The new inline HTML, wrapped code, long-label fallback, and reference section were also visually inspected in xterm.js on dark and light backgrounds at 40 columns.

For the heading example below, output now stays in order:

```text
Install `rustdown` with Cargo (https://example.com)
===================================================
```

Nested ordered lists now restart independently and align with their parent's content:

```text
1. First
   1. Child A
   2. Child B
2. Second
   1. Child C
   2. Child D
```

Code remains unwrapped by default; `--wrap-code` enables whitespace-preserving wrapping. Unsupported inline HTML remains visible. Use the explicit `html` subcommand for whole-document conversion. See the [README](../README.md) for the current options and limitations.

A parser adapter now accepts closing fences with trailing tabs. It uses an offset-preserving parser view and restores code, inline code, and HTML from the original input, so tabs in code remain intact. Regression cases cover nested blocks, longer fences containing shorter delimiters, and original-fence output.

The highest-value improvement is preserving structure when Markdown features are combined. Simple paragraphs, headings, inline styles, and code blocks work independently, but composition frequently breaks. Correct those paths before adding a richer visual theme.

## Reproduce

```sh
cargo run -- references/rendering-audit.md
cargo run -- --preserve-fences references/rendering-audit.md
cargo run -- references/test.md
```

Try narrow and wide terminal windows. The new [sample](../references/rendering-audit.md) intentionally combines features and contains long lines, CJK text, a combining accent, and a joined emoji.

This audit used all three existing reference files and 20 isolated probes, captured through a pseudo-terminal with ANSI color enabled. Width probes used actual PTY column sizes of 40, 80, and 120, together with matching `COLUMNS` values. Findings come from output text, ANSI style sequences, and source inspection; desktop theme contrast and font-specific emoji appearance have not been visually verified.

## Confirmed defects

### 1. Inline content escapes its containing heading or table

Input:

```md
# Install `rustdown` with [Cargo](https://example.com)
```

Actual output, with ANSI colors removed:

```text
`rustdown` (https://example.com)
Install  with Cargo
===================
```

Table links similarly print their URL before the table. Images print an empty image label before the table, while their alt text appears inside a cell. Footnote references escape the cell too. Heading images have the same ordering problem.

Cause: plain text is buffered, while inline code, link endings, image endings, and footnote references can write directly to the output. See `src/render/mod.rs` (`write_event_code`, `end_link`, `end_image`, `write_text`) and `src/markdown.rs` (`FootnoteReference`).

**Change:** route all inline content through a common representation that retains text and styles until its containing block is laid out. Headings and cells must receive complete inline content before rendering.

### 2. Lists lose numbering, indentation, and consistent spacing

- `7. Seven` / `8. Eight` renders as `1. Seven` / `2. Eight`.
- Two sibling nested lists numbered 1–2 render as 1–2 and then 3–4.
- Hard-break continuations and later paragraphs return to column zero.
- Nested ordered markers use two spaces per depth, so they do not align with the parent's content column.
- Ending a nested list adds extra blank lines; the existing `references/test.md` demonstrates this.
- Nested tasks get indentation twice: `  *   ☑ Nested completed`.

Cause: `start_list` ignores the supplied starting number, counters are not popped with each list, `end_item` always emits a newline, and continuation indentation is not represented. `render_task_list_item` adds indentation after the marker already did so.

**Change:** keep a stack of list frames with the current number, marker width, and content indentation. Apply that indentation to every continuation and child block. Emit spacing according to block context. For unordered tasks, a checkbox alone would be visually cleaner than a bullet plus checkbox; retain numbering for ordered tasks.

### 3. Quotes lose their structure

Only the first paragraph reliably gets a rail. Hard breaks lose the rail, nested quotes do not accumulate rails, and ending an inner quote clears the outer quote's state. Lists and code blocks inside quotes escape the border.

For example, `> first` followed by a hard break and `> second` produces:

```text
│ first
second
```

Cause: quote state is a boolean and prefixes are emitted on start/soft-break events, rather than on every output line.

**Change:** track quote depth and render prefixes centrally for each physical line, including nested blocks and deliberately blank quote lines.

### 4. Layout ignores terminal width and Unicode display width

The width probe produced identical bytes at 40, 80, and 120 columns. Paragraphs and long table cells are unbounded, and horizontal rules are always 60 characters. Terminal wrapping therefore cannot preserve list indentation, quote rails, or table borders.

`日本語` gets a three-character heading underline even though it normally occupies six terminal columns. Table borders also drift for CJK, combining accents, and joined emoji because widths use `chars().count()` and scalar-count padding.

**Change:** introduce a rendering width, with terminal detection in the CLI and an explicit option for library callers. Wrap styled prose with hanging indentation. Allocate table widths within the available space and wrap cell contents; use a readable fallback when columns cannot fit. Preserve code whitespace, with an explicit policy for long code lines. Size rules to the available content width.

Measure terminal columns rather than scalar counts; [`UnicodeWidthStr::width`](https://docs.rs/unicode-width/0.2.2/unicode_width/trait.UnicodeWidthStr.html) provides a practical basis. Actual emoji width can still depend on the terminal/font.

### 5. Table alignment and styling are discarded

Left, center, and right alignment all render left-aligned. Bold and italic cells become plain text even in the ANSI capture; headers are visually indistinguishable from body cells except for the separator.

Cause: `Tag::Table(_)` discards alignment metadata, and cells buffer plain strings. The parser already supplies [column alignment](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/enum.Tag.html#variant.Table).

**Change:** preserve alignments and styled cell content, right-align numeric columns when requested, emphasize headers, and make borders visually quieter than content. Offer compact Unicode borders with a plain-text fallback.

### 6. Footnotes have references but no identifiable definitions

`Text[^alpha] and other[^beta].` keeps the references, but definitions render as two ordinary, unlabeled paragraphs. Readers cannot reliably match them back to the references.

Cause: `Tag::FootnoteDefinition` and its end event are ignored.

**Change:** collect labeled definitions into a separated notes section, retain their inline/block formatting, and use consistent labels or reference-order numbering. Preserve multiple paragraphs within each definition.

### 7. Mixed HTML has inconsistent behavior and can lose separation

- `Before<br>after` becomes `Beforeafter`.
- `<kbd>Enter</kbd>` loses its semantic styling.
- An HTML block following a Markdown heading prints raw tags.
- A document starting with `<div>Intro</div>` is treated wholly as HTML, so subsequent Markdown headings and emphasis can become literal text on a collapsed line.

Cause: `InlineHtml` is ignored, block HTML is printed, and `app.rs` classifies the entire input based on its opening tag.

**Change:** decide and document a consistent mixed-HTML policy. At minimum handle line breaks and common inline semantics, render or visibly preserve unsupported content, and avoid silently reinterpreting a Markdown document solely because it starts with an HTML tag. Keep the explicit `html` command for intentional whole-document conversion.

### 8. Nested inline styles do not compose

In `*outer **bold *inner* bold** outer again*`, closing the inner emphasis clears the outer emphasis. ANSI output confirms that the final outer text is no longer italic. Inline code inside bold or strikethrough also loses the surrounding style.

Cause: boolean style flags cannot represent nested instances, and inline code creates a fresh style independent of its context.

**Change:** use scoped style state or nesting counters and compose inline-code styling with its surrounding styles. Add assertions against recorded `ColorSpec` values as well as plain text; the current colorless writer cannot catch these regressions.

## Visual improvements after structural fixes

- **Calmer hierarchy:** reserve strong emphasis for headings; choose one primary accent, subdued metadata, and consistent spacing. H1/H2 currently use underlines while H3–H6 retain literal hashes and each level has a different color. A coherent hierarchy would be easier to scan. Validate on both light and dark terminal themes before choosing defaults.
- **Readable code blocks:** give code a subtle gutter or frame, a small language label, and optional syntax highlighting with a plain fallback. Inline code can use a subtle contrasting treatment instead of always being red. Preserve whitespace and copyability.
- **Less URL clutter:** avoid repeating an autolink as `https://example.com (https://example.com)`. Provide a deliberate link policy for label plus URL, terminal hyperlinks, or a reference section. Make image labels compact, retain alt text, and handle linked images coherently.
- **GitHub alerts:** `[!NOTE]`, `[!WARNING]`, and related markers currently remain literal quote text. Add distinct labels and restrained accents after quote layout is fixed. The installed parser supports these via [`ENABLE_GFM`](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/struct.Options.html#associatedconstant.ENABLE_GFM).
- **Predictable output controls:** add `--width` and `--color auto|always|never`, and corresponding library options while preserving the existing convenience API. With `TERM=xterm-256color`, captured pipe output currently still contains ANSI escapes. Define how redirection and `NO_COLOR` interact with explicit color choices.

## Adjacent findings

- `--preserve-fences` normalizes `~~~~rust title=demo` to three backticks plus `rust`, losing the original delimiter and remaining info string. Either preserve source delimiters/metadata or document the flag as showing fences rather than preserving originals. Test longer fences containing triple backticks too.
- Terminal escape filtering only covers raw block HTML. A fenced code probe containing an ESC clear-screen sequence preserved that sequence verbatim in captured output. Apply a consistent policy to user-provided text, code, URLs, and labels before output; keep renderer-generated styles separate. The probe was inspected as escaped bytes, not replayed into a terminal.
- README quick-start examples claim `cargo run` renders a built-in sample, but interactive invocation requires input. Update usage docs alongside new options.

## Suggested implementation order

1. **Preserve content and structure:** common inline representation; list/quote/style stacks; footnote labels; consistent HTML handling; control-character handling. Add public-API regression tests for the concrete failures above.
2. **Add a layout layer:** terminal-column measurement, content width, wrapping, line prefixes, table alignment, and consistent block spacing. Test 40/80/120 columns and Unicode through explicit options so tests do not depend on the host terminal.
3. **Polish presentation:** heading palette, code treatment/highlighting, alerts, link/image presentation, output options, and usage docs. Inspect real terminals on light and dark themes and verify non-color output.

This can remain a streaming parser with block-local buffers; it does not require replacing pulldown-cmark or building a full-document AST. The key boundary is parsing events into complete styled block content before measuring and writing lines. Keep writer injection and I/O error propagation throughout.

## Validation and limits

- `cargo test`: passed; only two integration tests exist, covering a simple heading and basic backtick fences.
- Existing `simple.md`, `test.md`, and `full.md`: rendered locally through a PTY; output and ANSI sequences inspected.
- 20 isolated probes: exercised inline composition, tables, Unicode, lists, quotes, footnotes, HTML, links, alerts, styles, fences, and width.
- Width probe: identical output at 40/80/120 PTY columns.
- `cargo fmt --check`: fails on pre-existing formatting in `src/app.rs`; no application source was changed for this audit.
- No cross-terminal screenshots, light-theme contrast checks, performance benchmarks, or implemented-fix validation were performed.
