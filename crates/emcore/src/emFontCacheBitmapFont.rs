// SPLIT: Split from emFontCache.h — bitmap font implementation extracted
//! Text measurement utilities (character/line counting).
//!
//! The actual glyph rendering is handled by `em_font`, which uses the
//! Eagle Mode grayscale font atlas. This module retains `measure_formatted`
//! for text layout calculations.

/// Measure text width in characters, handling formatted text (tabs and newlines).
/// Returns `(max_columns, row_count)`.
pub(crate) fn measure_formatted(text: &str) -> (usize, usize) {
    let mut max_cols = 0usize;
    let mut rows = 1usize;
    let mut col = 0usize;

    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                // \r\n or lone \r both count as one newline
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                max_cols = max_cols.max(col);
                col = 0;
                rows += 1;
            }
            '\n' => {
                max_cols = max_cols.max(col);
                col = 0;
                rows += 1;
            }
            '\t' => {
                // Align to next multiple of 8
                col = (col / 8 + 1) * 8;
            }
            _ => {
                col += 1;
            }
        }
    }
    max_cols = max_cols.max(col);
    (max_cols, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_formatted_single_line() {
        let (cols, rows) = measure_formatted("Hello");
        assert_eq!(cols, 5);
        assert_eq!(rows, 1);
    }

    #[test]
    fn measure_formatted_multiline() {
        let (cols, rows) = measure_formatted("Hello\nWorld!\nFoo");
        assert_eq!(cols, 6); // "World!" is longest
        assert_eq!(rows, 3);
    }

    #[test]
    fn measure_formatted_tabs() {
        let (cols, _) = measure_formatted("A\tB");
        assert_eq!(cols, 9); // A at 0, tab to 8, B at 8 -> 9 total
    }

    #[test]
    fn measure_formatted_crlf() {
        let (cols, rows) = measure_formatted("A\r\nB");
        assert_eq!(cols, 1);
        assert_eq!(rows, 2);
    }
}
