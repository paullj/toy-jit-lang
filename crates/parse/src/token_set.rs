use lex::TokenKind;

/// Efficient set of TokenKind implemented as a bitset.
/// Supports up to 64 token kinds.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenSet(u64);

impl TokenSet {
    pub const EMPTY: TokenSet = TokenSet(0);

    pub const fn new(kinds: &[TokenKind]) -> Self {
        let mut bits = 0u64;
        let mut i = 0;
        while i < kinds.len() {
            bits |= 1 << (kinds[i] as u32);
            i += 1;
        }
        TokenSet(bits)
    }

    pub const fn single(kind: TokenKind) -> Self {
        TokenSet(1 << (kind as u32))
    }

    pub const fn union(self, other: TokenSet) -> TokenSet {
        TokenSet(self.0 | other.0)
    }

    pub const fn contains(&self, kind: TokenKind) -> bool {
        self.0 & (1 << (kind as u32)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_contains_nothing() {
        assert!(!TokenSet::EMPTY.contains(TokenKind::Integer));
        assert!(!TokenSet::EMPTY.contains(TokenKind::Identifier));
    }

    #[test]
    fn single_element_set() {
        let set = TokenSet::single(TokenKind::Integer);
        assert!(set.contains(TokenKind::Integer));
        assert!(!set.contains(TokenKind::Identifier));
    }

    #[test]
    fn multi_element_set() {
        let set = TokenSet::new(&[TokenKind::Integer, TokenKind::Float, TokenKind::Identifier]);
        assert!(set.contains(TokenKind::Integer));
        assert!(set.contains(TokenKind::Float));
        assert!(set.contains(TokenKind::Identifier));
        assert!(!set.contains(TokenKind::Plus));
    }

    #[test]
    fn union_sets() {
        let a = TokenSet::new(&[TokenKind::Integer, TokenKind::Float]);
        let b = TokenSet::new(&[TokenKind::Identifier, TokenKind::String]);
        let union = a.union(b);

        assert!(union.contains(TokenKind::Integer));
        assert!(union.contains(TokenKind::Float));
        assert!(union.contains(TokenKind::Identifier));
        assert!(union.contains(TokenKind::String));
        assert!(!union.contains(TokenKind::Plus));
    }
}
