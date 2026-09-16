use super::{Renderer, layout::*};
use crate::markdown::Node;
use pulldown_cmark::{Alignment, Tag};

impl Renderer<'_> {
    pub(super) fn table(
        &self,
        nodes: &[Node<'_>],
        alignments: &[Alignment],
        columns: usize,
    ) -> Lines {
        let mut rows: Vec<Vec<Line>> = Vec::new();
        for node in nodes {
            if let Node::Element(Tag::TableHead | Tag::TableRow, children, _) = node {
                let header = matches!(node, Node::Element(Tag::TableHead, _, _));
                let cells = children
                    .iter()
                    .filter_map(|cell| {
                        if let Node::Element(Tag::TableCell, children, _) = cell {
                            let mut content = self.inline(
                                children,
                                Style {
                                    bold: header,
                                    ..Style::default()
                                },
                            );
                            if let Some(first) = content.first_mut() {
                                first.text = first.text.trim_start().to_string();
                            }
                            if let Some(last) = content.last_mut() {
                                last.text = last.text.trim_end().to_string();
                            }
                            Some(content)
                        } else {
                            None
                        }
                    })
                    .collect();
                rows.push(cells);
            }
        }
        let count = alignments.len();
        if count == 0 || rows.is_empty() {
            return Vec::new();
        }
        // At least three content columns per cell; narrow tables become labeled records.
        if count.saturating_mul(6).saturating_add(1) > columns {
            let mut out = Vec::new();
            let header = &rows[0];
            if rows.len() == 1 {
                for cell in header {
                    join(&mut out, wrap(cell, columns), false);
                }
            }
            for row in rows.iter().skip(1) {
                let mut record = Vec::new();
                for (i, cell) in row.iter().enumerate() {
                    let label = header.get(i).cloned().unwrap_or_default();
                    record.extend(wrap(&label, columns));
                    if columns >= 4 {
                        record.extend(prefix(wrap(cell, columns - 2), plain("  "), plain("  ")));
                    } else {
                        record.extend(wrap(cell, columns));
                    }
                }
                join(&mut out, record, true);
            }
            return out;
        }
        let mut desired = vec![3; count];
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                desired[i] = desired[i].max(
                    wrap(cell, usize::MAX)
                        .iter()
                        .map(|l| width(l))
                        .max()
                        .unwrap_or(0)
                        .min(columns),
                );
            }
        }
        // Cap wide columns evenly while preserving naturally narrow columns.
        // Binary search avoids work proportional to input cell lengths.
        let available = columns - (count * 3 + 1);
        let (mut low, mut high) = (3, columns);
        while low < high {
            let cap = low + (high - low).div_ceil(2);
            if desired.iter().map(|n| (*n).min(cap)).sum::<usize>() <= available {
                low = cap;
            } else {
                high = cap - 1;
            }
        }
        let mut widths: Vec<_> = desired.iter().map(|n| (*n).min(low)).collect();
        let mut remaining = available - widths.iter().sum::<usize>();
        for i in 0..count {
            if remaining > 0 && widths[i] < desired[i] {
                widths[i] += 1;
                remaining -= 1;
            }
        }
        let border = |left: &str, middle: &str, right: &str| -> Line {
            let dash = if self.options.ascii { "-" } else { "─" };
            let cells: Vec<_> = widths.iter().map(|n| dash.repeat(n + 2)).collect();
            vec![span(
                format!("{left}{}{right}", cells.join(middle)),
                Style::muted(),
            )]
        };
        let mut out = vec![if self.options.ascii {
            border("+", "+", "+")
        } else {
            border("┌", "┬", "┐")
        }];
        let vertical = if self.options.ascii { "|" } else { "│" };
        for (row_index, row) in rows.iter().enumerate() {
            let cells: Vec<Lines> = (0..count)
                .map(|i| wrap(row.get(i).map(Vec::as_slice).unwrap_or(&[]), widths[i]))
                .collect();
            let height = cells.iter().map(Vec::len).max().unwrap_or(1);
            for y in 0..height {
                let mut line = vec![span(vertical, Style::muted())];
                for i in 0..count {
                    let content = cells[i].get(y).cloned().unwrap_or_default();
                    let padding = widths[i].saturating_sub(width(&content));
                    let left = match alignments[i] {
                        Alignment::Right => padding,
                        Alignment::Center => padding / 2,
                        _ => 0,
                    };
                    push(&mut line, plain(" ".repeat(1 + left)));
                    for s in content {
                        push(&mut line, s);
                    }
                    push(&mut line, plain(" ".repeat(1 + padding - left)));
                    push(&mut line, span(vertical, Style::muted()));
                }
                out.push(line);
            }
            if row_index == 0 {
                out.push(if self.options.ascii {
                    border("+", "+", "+")
                } else {
                    border("├", "┼", "┤")
                });
            }
        }
        out.push(if self.options.ascii {
            border("+", "+", "+")
        } else {
            border("└", "┴", "┘")
        });
        out
    }
}
