use syntax::TextRange;

/// 0-indexed position (line + UTF-16 column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range defined by start and end positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Maps between byte offsets and positions (line + UTF-16 column).
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<u32>,
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

    /// Convert byte offset to Position (0-indexed line + UTF-16 column).
    pub fn position(&self, offset: u32) -> Position {
        let line = self.line_of_offset(offset);
        let line_start = self.line_starts[line as usize];
        let col_byte = offset - line_start;

        let line_end = self
            .line_starts
            .get(line as usize + 1)
            .copied()
            .unwrap_or(self.text.len() as u32);
        let line_text = &self.text[line_start as usize..line_end as usize];

        let col_utf16 = utf8_to_utf16_col(line_text, col_byte as usize);

        Position::new(line, col_utf16)
    }

    /// Convert Position to byte offset.
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

        let col_byte = utf16_to_utf8_col(line_text, pos.character as usize);

        line_start + col_byte as u32
    }

    /// Convert TextRange to Range.
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

fn utf8_to_utf16_col(line: &str, byte_col: usize) -> u32 {
    let byte_col = byte_col.min(line.len());
    line[..byte_col].encode_utf16().count() as u32
}

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
        let idx = LineIndex::new("a\u{1F600}b");
        assert_eq!(idx.position(0), Position::new(0, 0));
        assert_eq!(idx.position(1), Position::new(0, 1));
        assert_eq!(idx.position(5), Position::new(0, 3));
    }

    #[test]
    fn test_roundtrip() {
        let text = "foo\nbar\u{1F600}baz";
        let idx = LineIndex::new(text);
        // Only test valid char boundaries
        for (offset, _) in text.char_indices() {
            let pos = idx.position(offset as u32);
            let back = idx.offset(pos);
            assert_eq!(offset as u32, back, "roundtrip failed for offset {offset}");
        }
    }
}
