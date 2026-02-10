fn chunk_str(s: &str, size: usize) -> Vec<String> {
    s.chars()
        .collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.iter().collect::<String>())
        .collect()
}

pub fn present_table(rows: &[Vec<String>]) -> String {
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if cols == 0 {
        return String::new();
    }

    let mut widths = vec![0usize; cols];
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            widths[i] = widths[i].max(val.len());
        }
    }

    let mut out = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        let mut line_parts = Vec::new();

        for (i, cell) in row.iter().enumerate() {
            let padded = format!("{:width$}", cell, width = widths[i]);
            line_parts.push(padded);
        }

        out.push(line_parts.join(" | "));

        if row_idx == 0 {
            let divider_parts: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
            out.push(divider_parts.join("-+-"));
        }
    }

    out.join("\n")
}
