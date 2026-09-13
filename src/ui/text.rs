//! Display-width-aware text wrapping and padding.

use unicode_width::UnicodeWidthStr;

/// Hard-wraps each line at `width` display columns (no word breaking, tabs as
/// 4 spaces).
pub(super) fn wrap_lines(lines: &[String], width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for line in lines {
        let line = line.replace('\t', "    ");
        let mut cur = String::new();
        let mut w = 0;
        for ch in line.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > width && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
                w = 0;
            }
            cur.push(ch);
            w += cw;
        }
        out.push(cur);
    }
    out
}

/// Pads or truncates `s` to exactly `width` display columns.
pub(super) fn pad(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > width {
            break;
        }
        out.push(ch);
        w += cw;
    }
    while w < width {
        out.push(' ');
        w += 1;
    }
    debug_assert_eq!(out.width(), width);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_by_display_width() {
        let lines = vec![
            "abcdefgh".to_string(),
            "".to_string(),
            "日本語テキスト".to_string(),
        ];
        assert_eq!(
            wrap_lines(&lines, 6),
            vec!["abcdef", "gh", "", "日本語", "テキス", "ト"]
        );
    }
}
