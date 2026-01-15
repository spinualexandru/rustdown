use crate::render::MarkdownRenderer;
use std::io;
use termcolor::WriteColor;

pub(crate) fn render_table<'a, W: WriteColor + ?Sized>(
    renderer: &mut MarkdownRenderer<'a, W>,
) -> io::Result<()> {
    if renderer.table_rows.is_empty() {
        return Ok(());
    }

    // Compute column count and widths with proper cell trimming
    let cols = renderer
        .table_rows
        .iter()
        .map(|r| r.len())
        .max()
        .unwrap_or(0);
    let mut widths = vec![0usize; cols];

    // Trim cells and calculate proper widths
    let mut trimmed_rows = Vec::new();
    for row in &renderer.table_rows {
        let mut trimmed_row = Vec::new();
        for (i, cell) in row.iter().enumerate() {
            let trimmed = cell.trim();
            trimmed_row.push(trimmed.to_string());
            if i < widths.len() {
                widths[i] = widths[i].max(trimmed.chars().count());
            }
        }
        trimmed_rows.push(trimmed_row);
    }

    // Ensure minimum width for readability
    for width in &mut widths {
        *width = (*width).max(3);
    }

    // Print rows with improved alignment
    for (ri, row) in trimmed_rows.iter().enumerate() {
        let mut out_cells: Vec<String> = Vec::new();
        for i in 0..cols {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let padded = format!(" {:<width$} ", cell, width = widths[i]);
            out_cells.push(padded);
        }
        writeln!(renderer.output, "|{}|", out_cells.join("|"))?;

        // After header row, print separator with proper alignment
        if renderer.table_header_count > 0 && ri + 1 == renderer.table_header_count {
            let mut sep_cells: Vec<String> = Vec::new();
            for &width in &widths {
                sep_cells.push(format!(" {} ", "-".repeat(width)));
            }
            writeln!(renderer.output, "|{}|", sep_cells.join("|"))?;
        }
    }

    Ok(())
}
