#[macro_export]
macro_rules! concat_kinds {
    // Two slices - base case with deduplication
    ($s1:expr, $s2:expr) => {{
        const fn concat_and_dedup_two(s1: &[lex::TokenKind], s2: &[lex::TokenKind]) -> ([lex::TokenKind; 64], usize) {
            let mut result = [lex::TokenKind::Whitespace; 64];
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
                        // Trivia
                        (lex::TokenKind::Whitespace, lex::TokenKind::Whitespace) => true,
                        (lex::TokenKind::Comment, lex::TokenKind::Comment) => true,
                        (lex::TokenKind::NewLine, lex::TokenKind::NewLine) => true,
                        // Brackets
                        (lex::TokenKind::LeftParenthesis, lex::TokenKind::LeftParenthesis) => true,
                        (lex::TokenKind::RightParenthesis, lex::TokenKind::RightParenthesis) => true,
                        // Punctuation
                        (lex::TokenKind::Comma, lex::TokenKind::Comma) => true,
                        (lex::TokenKind::Colon, lex::TokenKind::Colon) => true,
                        // Int arithmetic
                        (lex::TokenKind::Plus, lex::TokenKind::Plus) => true,
                        (lex::TokenKind::Minus, lex::TokenKind::Minus) => true,
                        (lex::TokenKind::Asterisk, lex::TokenKind::Asterisk) => true,
                        (lex::TokenKind::Slash, lex::TokenKind::Slash) => true,
                        (lex::TokenKind::Percent, lex::TokenKind::Percent) => true,
                        // Float arithmetic
                        (lex::TokenKind::PlusDot, lex::TokenKind::PlusDot) => true,
                        (lex::TokenKind::MinusDot, lex::TokenKind::MinusDot) => true,
                        (lex::TokenKind::AsteriskDot, lex::TokenKind::AsteriskDot) => true,
                        (lex::TokenKind::SlashDot, lex::TokenKind::SlashDot) => true,
                        // Comparison
                        (lex::TokenKind::Equals, lex::TokenKind::Equals) => true,
                        (lex::TokenKind::EqualsEquals, lex::TokenKind::EqualsEquals) => true,
                        (lex::TokenKind::NotEquals, lex::TokenKind::NotEquals) => true,
                        (lex::TokenKind::Bang, lex::TokenKind::Bang) => true,
                        (lex::TokenKind::GreaterThan, lex::TokenKind::GreaterThan) => true,
                        (lex::TokenKind::LessThan, lex::TokenKind::LessThan) => true,
                        (lex::TokenKind::GreaterThanOrEqual, lex::TokenKind::GreaterThanOrEqual) => true,
                        (lex::TokenKind::LessThanOrEqual, lex::TokenKind::LessThanOrEqual) => true,
                        // Float comparison
                        (lex::TokenKind::GreaterThanDot, lex::TokenKind::GreaterThanDot) => true,
                        (lex::TokenKind::LessThanDot, lex::TokenKind::LessThanDot) => true,
                        (lex::TokenKind::GreaterThanOrEqualDot, lex::TokenKind::GreaterThanOrEqualDot) => true,
                        (lex::TokenKind::LessThanOrEqualDot, lex::TokenKind::LessThanOrEqualDot) => true,
                        // Keywords
                        (lex::TokenKind::And, lex::TokenKind::And) => true,
                        (lex::TokenKind::Or, lex::TokenKind::Or) => true,
                        (lex::TokenKind::True, lex::TokenKind::True) => true,
                        (lex::TokenKind::False, lex::TokenKind::False) => true,
                        // Literals
                        (lex::TokenKind::Identifier, lex::TokenKind::Identifier) => true,
                        (lex::TokenKind::BinaryInteger, lex::TokenKind::BinaryInteger) => true,
                        (lex::TokenKind::OctalInteger, lex::TokenKind::OctalInteger) => true,
                        (lex::TokenKind::HexInteger, lex::TokenKind::HexInteger) => true,
                        (lex::TokenKind::Integer, lex::TokenKind::Integer) => true,
                        (lex::TokenKind::Float, lex::TokenKind::Float) => true,
                        (lex::TokenKind::FloatExponent, lex::TokenKind::FloatExponent) => true,
                        (lex::TokenKind::String, lex::TokenKind::String) => true,
                        (lex::TokenKind::MultiLineString, lex::TokenKind::MultiLineString) => true,
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

        const RESULT_WITH_LEN: ([lex::TokenKind; 64], usize) = concat_and_dedup_two($s1, $s2);

        // Create a properly sized slice (support up to 48 elements)
        const RESULT: &[lex::TokenKind] = {
            const R: &[lex::TokenKind; 64] = &RESULT_WITH_LEN.0;
            const L: usize = RESULT_WITH_LEN.1;
            match L {
                0 => &[],
                1 => &[R[0]],
                2 => &[R[0], R[1]],
                3 => &[R[0], R[1], R[2]],
                4 => &[R[0], R[1], R[2], R[3]],
                5 => &[R[0], R[1], R[2], R[3], R[4]],
                6 => &[R[0], R[1], R[2], R[3], R[4], R[5]],
                7 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6]],
                8 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7]],
                9 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8]],
                10 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9]],
                11 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10]],
                12 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11]],
                13 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12]],
                14 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13]],
                15 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14]],
                16 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15]],
                17 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16]],
                18 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17]],
                19 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18]],
                20 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19]],
                21 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20]],
                22 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21]],
                23 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22]],
                24 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23]],
                25 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24]],
                26 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25]],
                27 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26]],
                28 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27]],
                29 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28]],
                30 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29]],
                31 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30]],
                32 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31]],
                33 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32]],
                34 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33]],
                35 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34]],
                36 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35]],
                37 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36]],
                38 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37]],
                39 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38]],
                40 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39]],
                41 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40]],
                42 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41]],
                43 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42]],
                44 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42], R[43]],
                45 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42], R[43], R[44]],
                46 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42], R[43], R[44], R[45]],
                47 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42], R[43], R[44], R[45], R[46]],
                48 => &[R[0], R[1], R[2], R[3], R[4], R[5], R[6], R[7], R[8], R[9], R[10], R[11], R[12], R[13], R[14], R[15], R[16], R[17], R[18], R[19], R[20], R[21], R[22], R[23], R[24], R[25], R[26], R[27], R[28], R[29], R[30], R[31], R[32], R[33], R[34], R[35], R[36], R[37], R[38], R[39], R[40], R[41], R[42], R[43], R[44], R[45], R[46], R[47]],
                _ => panic!("Too many unique tokens - increase macro capacity"),
            }
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
