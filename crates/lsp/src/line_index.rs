use syntax::TextRange;
use tower_lsp::lsp_types::{Position, Range};

/// Maps between byte offsets and LSP positions (line + UTF-16 column).
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of the start of each line.
    line_starts: Vec<u32>,
    /// Source text (needed for UTF-16 conversion).
    text: String,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self {
            line_starts,
            text: text.to_string(),
        }
    }

    /// Convert byte offset to LSP Position (0-indexed line + UTF-16 column).
    pub fn position(&self, offset: u32) -> Position {
        let line = self.line_of_offset(offset);
        let line_start = self.line_starts[line as usize];
        let col_byte = offset - line_start;

        // Get the line text up to the offset
        let line_end = self
            .line_starts
            .get(line as usize + 1)
            .copied()
            .unwrap_or(self.text.len() as u32);
        let line_text = &self.text[line_start as usize..line_end as usize];

        // Convert byte column to UTF-16 code units
        let col_utf16 = utf8_to_utf16_col(line_text, col_byte as usize);

        Position::new(line, col_utf16)
    }

    /// Convert LSP Position to byte offset.
    #[allow(dead_code)]
    pub fn offset(&self, pos: Position) -> u32 {
        let line = pos.line as usize;
        if line >= self.line_starts.len() {
            return self.text.len() as u32;
        }

        let line_start = self.line_starts[line];
        let line_end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len() as u32);
        let line_text = &self.text[line_start as usize..line_end as usize];

        // Convert UTF-16 column to byte offset
        let col_byte = utf16_to_utf8_col(line_text, pos.character as usize);

        line_start + col_byte as u32
    }

    /// Convert TextRange to LSP Range.
    #[allow(dead_code)]
    pub fn range(&self, text_range: TextRange) -> Range {
        Range::new(
            self.position(text_range.start().into()),
            self.position(text_range.end().into()),
        )
    }

    fn line_of_offset(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line as u32,
            Err(line) => (line - 1) as u32,
        }
    }
}

/// Convert byte column to UTF-16 code units.
fn utf8_to_utf16_col(line: &str, byte_col: usize) -> u32 {
    let byte_col = byte_col.min(line.len());
    line[..byte_col].encode_utf16().count() as u32
}

/// Convert UTF-16 column to byte offset within line.
#[allow(dead_code)]
fn utf16_to_utf8_col(line: &str, utf16_col: usize) -> usize {
    let mut utf16_count = 0;
    for (i, c) in line.char_indices() {
        if utf16_count >= utf16_col {
            return i;
        }
        utf16_count += c.len_utf16();
    }
    line.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        let idx = LineIndex::new("hello\nworld");
        assert_eq!(idx.position(0), Position::new(0, 0));
        assert_eq!(idx.position(5), Position::new(0, 5));
        assert_eq!(idx.position(6), Position::new(1, 0));
        assert_eq!(idx.position(11), Position::new(1, 5));
    }

    #[test]
    fn test_utf16() {
        // '😀' is 4 bytes in UTF-8, 2 code units in UTF-16
        let idx = LineIndex::new("a😀b");
        assert_eq!(idx.position(0), Position::new(0, 0)); // 'a'
        assert_eq!(idx.position(1), Position::new(0, 1)); // start of emoji
        assert_eq!(idx.position(5), Position::new(0, 3)); // 'b' (1 + 2 utf16 units)
    }

    #[test]
    fn test_roundtrip() {
        let idx = LineIndex::new("foo\nbar😀baz");
        for offset in 0..15 {
            let pos = idx.position(offset);
            let back = idx.offset(pos);
            assert_eq!(offset, back, "roundtrip failed for offset {offset}");
        }
    }
}
