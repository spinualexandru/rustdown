# Rendering audit

## Install `rustdown` with [Cargo](https://example.com)

This heading should keep its code and link together.

## Inline formatting

*Outer emphasis **bold with *inner emphasis* and still bold** and still outer emphasis.*

**Bold `code` stays bold** and ~~removed `code` stays removed~~.

An automatic link: <https://example.com>.

Before<br>after and press <kbd>Enter</kbd>.

## Ordered list starts

7. Seven
8. Eight

## Sibling nested lists

1. First parent
   1. Child A
   2. Child B
2. Second parent
   1. Child C
   2. Child D

## List continuation and spacing

- First line with a hard break.  
  This line belongs to the same item.

  This paragraph also belongs to the same item.

  > A quoted child.

  ```rust
  let answer = 42;
  ```

- Next item.

## Tasks

- [x] Completed task
- [ ] Pending task
  - [x] Nested completed task
  - [ ] Nested pending task

## Quotes with block children

> First paragraph.
>
> Second paragraph.
>
> > Nested quote.
>
> Back to the outer quote.
>
> - Child one
> - Child two
>
> ```rust
> let answer = 42;
> ```

> A line with a hard break.  
> This line still needs its quote border.

## Tables with rich content

| Link | Image | Note |
| :--- | :---: | ---: |
| [docs](https://example.com) | ![logo](logo.png) | Reference[^table] |
| **Bold** | *Italic* | `Code` |

## Table alignment

| Name | Center | Amount |
| :--- | :---: | ---: |
| a | x | 1 |
| longer | yyy | 12345 |

## 日本語

| Name | Value |
| --- | --- |
| 日本語 | 12 |
| é | 3 |
| 👩‍💻 | 4 |
| abc | 5 |

## Footnotes

First reference[^alpha] and a different reference[^beta].

[^table]: A footnote referenced inside a table.
[^alpha]: First footnote, with **formatting**.
[^beta]: Second footnote.

## Alerts

> [!WARNING]
> Save your changes before continuing.

## Code fence preservation

~~~~rust title=demo
let answer = 42;
~~~~

## Mixed HTML

<div>Visible <b>bold</b> text inside an HTML block.</div>

## Narrow terminal layout

- This is a long list item that should retain its indentation when it wraps. This is a long list item that should retain its indentation when it wraps. This is a long list item that should retain its indentation when it wraps.

> A long quoted sentence should keep the quote rail across wrapped lines. A long quoted sentence should keep the quote rail across wrapped lines. A long quoted sentence should keep the quote rail across wrapped lines.

| Description | Status |
| --- | --- |
| Long cell contents with enough words to exceed a narrow terminal and reveal whether table borders remain aligned across wrapped lines. | OK |

---

## Inline HTML and links

Read <a href="https://example.com?one=1&amp;two=2">**the HTML link**</a>, view <img src="logo.png" alt="A &amp; B logo">, and compare H<sub>2</sub>O with x<sup>n + 1</sup>.

[Repeated link](https://example.com) and [the same destination](https://example.com) should share one reference in reference mode.

## Deep nesting and long labels

> > > > > A deeply nested quote stays readable at narrow widths.

123456789. A list marker that can exceed the available indentation budget.

A note with a long label[^a_very_long_footnote_identifier].

[^a_very_long_footnote_identifier]: The label moves above its body when necessary.

## Code wrapping and tabs

```rust
	let message = "A long code line should wrap only when wrapping is explicitly enabled.";
	println!("{message}");
```	

This paragraph follows a closing fence with a trailing tab.
