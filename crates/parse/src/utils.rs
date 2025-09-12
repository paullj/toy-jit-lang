#[macro_export]
macro_rules! concat_kinds {
    // Two slices - base case with deduplication
    ($s1:expr, $s2:expr) => {{
        const fn concat_and_dedup_two(s1: &[lex::TokenKind], s2: &[lex::TokenKind]) -> ([lex::TokenKind; 32], usize) {
            let mut result = [lex::TokenKind::Whitespace; 32];
            let mut len = 0;
            
            // Add all elements from first slice
            let mut i = 0;
            while i < s1.len() {
                result[len] = s1[i];
                len += 1;
                i += 1;
            }
            
            // Add elements from second slice, checking for duplicates
            let mut j = 0;
            while j < s2.len() {
                let token = s2[j];
                
                // Check if already exists
                let mut found = false;
                let mut check_idx = 0;
                while check_idx < len {
                    // Manual comparison since PartialEq isn't const
                    let is_same = match (result[check_idx], token) {
                        (lex::TokenKind::Whitespace, lex::TokenKind::Whitespace) => true,
                        (lex::TokenKind::Comment, lex::TokenKind::Comment) => true,
                        (lex::TokenKind::NewLine, lex::TokenKind::NewLine) => true,
                        (lex::TokenKind::LeftParenthesis, lex::TokenKind::LeftParenthesis) => true,
                        (lex::TokenKind::RightParenthesis, lex::TokenKind::RightParenthesis) => true,
                        (lex::TokenKind::LeftBrace, lex::TokenKind::LeftBrace) => true,
                        (lex::TokenKind::RightBrace, lex::TokenKind::RightBrace) => true,
                        (lex::TokenKind::Comma, lex::TokenKind::Comma) => true,
                        (lex::TokenKind::Plus, lex::TokenKind::Plus) => true,
                        (lex::TokenKind::Minus, lex::TokenKind::Minus) => true,
                        (lex::TokenKind::Asterisk, lex::TokenKind::Asterisk) => true,
                        (lex::TokenKind::Slash, lex::TokenKind::Slash) => true,
                        (lex::TokenKind::Equals, lex::TokenKind::Equals) => true,
                        (lex::TokenKind::ColonEquals, lex::TokenKind::ColonEquals) => true,
                        (lex::TokenKind::Function, lex::TokenKind::Function) => true,
                        (lex::TokenKind::Identifier, lex::TokenKind::Identifier) => true,
                        (lex::TokenKind::Integer, lex::TokenKind::Integer) => true,
                        _ => false,
                    };
                    if is_same {
                        found = true;
                        break;
                    }
                    check_idx += 1;
                }
                
                if !found {
                    result[len] = token;
                    len += 1;
                }
                j += 1;
            }
            
            (result, len)
        }
        
        const RESULT_WITH_LEN: ([lex::TokenKind; 32], usize) = concat_and_dedup_two($s1, $s2);
        
        // Create a properly sized slice
        const RESULT: &[lex::TokenKind] = if RESULT_WITH_LEN.1 == 0 {
            &[]
        } else if RESULT_WITH_LEN.1 == 1 {
            &[RESULT_WITH_LEN.0[0]]
        } else if RESULT_WITH_LEN.1 == 2 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1]]
        } else if RESULT_WITH_LEN.1 == 3 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2]]
        } else if RESULT_WITH_LEN.1 == 4 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3]]
        } else if RESULT_WITH_LEN.1 == 5 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4]]
        } else if RESULT_WITH_LEN.1 == 6 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4], RESULT_WITH_LEN.0[5]]
        } else if RESULT_WITH_LEN.1 == 7 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4], RESULT_WITH_LEN.0[5], RESULT_WITH_LEN.0[6]]
        } else if RESULT_WITH_LEN.1 == 8 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4], RESULT_WITH_LEN.0[5], RESULT_WITH_LEN.0[6], RESULT_WITH_LEN.0[7]]
        } else if RESULT_WITH_LEN.1 == 9 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4], RESULT_WITH_LEN.0[5], RESULT_WITH_LEN.0[6], RESULT_WITH_LEN.0[7], RESULT_WITH_LEN.0[8]]
        } else if RESULT_WITH_LEN.1 == 10 {
            &[RESULT_WITH_LEN.0[0], RESULT_WITH_LEN.0[1], RESULT_WITH_LEN.0[2], RESULT_WITH_LEN.0[3], RESULT_WITH_LEN.0[4], RESULT_WITH_LEN.0[5], RESULT_WITH_LEN.0[6], RESULT_WITH_LEN.0[7], RESULT_WITH_LEN.0[8], RESULT_WITH_LEN.0[9]]
        } else {
            panic!("Too many unique tokens - increase macro capacity")
        };
        RESULT
    }};
    
    // Three or more slices - recursive approach
    ($s1:expr, $s2:expr, $($rest:expr),+ $(,)?) => {
        concat_kinds!(concat_kinds!($s1, $s2), $($rest),+)
    };
}

// For when you need deduplication, manually specify unique items
#[macro_export]
macro_rules! unique_kinds {
    ($($token:expr),* $(,)?) => {
        &[$($token),*]
    };
}
