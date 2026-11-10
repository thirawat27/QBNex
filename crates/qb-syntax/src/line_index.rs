//! Byte offset <-> (line, UTF-16 column) conversion, as used by LSP.

/// Line start offsets of a text.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// Lines end at `\n`, `\r\n` or a lone `\r`.
    pub fn new(text: &str) -> Self {
        let b = text.as_bytes();
        let mut starts = vec![0];
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\n' => starts.push(i + 1),
                b'\r' if b.get(i + 1) != Some(&b'\n') => starts.push(i + 1),
                _ => {}
            }
            i += 1;
        }
        LineIndex { starts }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Zero-based (line, UTF-16 column) of a byte offset (clamped to the text).
    pub fn position(&self, text: &str, offset: usize) -> (u32, u32) {
        let mut offset = offset.min(text.len());
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let col: usize = text[self.starts[line]..offset].chars().map(char::len_utf16).sum();
        (line as u32, col as u32)
    }

    /// Byte offset of a zero-based (line, UTF-16 column); clamped to the line.
    pub fn offset(&self, text: &str, line: u32, col: u32) -> usize {
        let Some(&start) = self.starts.get(line as usize) else { return text.len() };
        let end = self.starts.get(line as usize + 1).copied().unwrap_or(text.len());
        let mut units = 0u32;
        for (i, ch) in text[start..end].char_indices() {
            if units >= col || ch == '\r' || ch == '\n' {
                return start + i;
            }
            units += ch.len_utf16() as u32;
        }
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = "ab\r\nc€d\re";
        let li = LineIndex::new(t);
        assert_eq!(li.line_count(), 3);
        assert_eq!(li.position(t, 5), (1, 1));
        assert_eq!(li.position(t, 8), (1, 2));
        assert_eq!(li.offset(t, 1, 2), 8);
        assert_eq!(li.offset(t, 2, 0), t.len() - 1);
        assert_eq!(li.offset(t, 0, 99), 2);
    }
}
