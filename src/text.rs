fn sanitize(text: &str, left: char, right: char) -> String {
    let mut result = String::with_capacity(text.len() + 2);
    result.push(left);
    for ch in text.chars() {
        if (' '..='~').contains(&ch) {
            result.push(ch);
        } else {
            result.push('?');
        }
    }
    result.push(right);
    result
}

/// Wrap printable ASCII text in double quotes, replacing other chars with '?'.
pub fn show(text: &str) -> String {
    sanitize(text, '"', '"')
}

/// Wrap printable ASCII text in square brackets, replacing other chars with '?'.
pub fn cell(text: &str) -> String {
    sanitize(text, '[', ']')
}

/// Format a key in a ten-character column followed by two spaces and its value.
pub fn line(key: &str, value: &str) -> String {
    let mut result = String::with_capacity(key.len().max(10) + 2 + value.len());
    result.push_str(key);
    for _ in key.chars().count()..10 {
        result.push(' ');
    }
    result.push_str("  ");
    result.push_str(value);
    result
}

/// Format a header and rows as a left-aligned table with two-space separators.
pub fn table(header: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let columns = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(header.len());

    let mut widths = vec![0; columns];
    for (index, value) in header.iter().enumerate() {
        widths[index] = widths[index].max(value.chars().count());
    }
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.chars().count());
        }
    }

    let mut result = Vec::with_capacity(rows.len() + 1);
    result.push(format_row(header.iter().copied(), &widths));
    for row in rows {
        result.push(format_row(row.iter().map(String::as_str), &widths));
    }
    result
}

fn format_row<'a>(cells: impl Iterator<Item = &'a str>, widths: &[usize]) -> String {
    let cells: Vec<&str> = cells.collect();
    let mut result = String::new();
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            result.push_str("  ");
        }
        let value = cells.get(index).copied().unwrap_or("");
        result.push_str(value);
        if index + 1 < widths.len() {
            for _ in value.chars().count()..*width {
                result.push(' ');
            }
        }
    }
    while result.ends_with(' ') {
        result.pop();
    }
    result
}
