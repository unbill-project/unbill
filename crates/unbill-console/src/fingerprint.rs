//! Shared display encoding for SHA-256 fingerprints.

use std::sync::OnceLock;

const SYMBOL_COUNT: usize = 22;

// sirno:witness:emoji-fingerprint:begin
fn alphabet() -> &'static [&'static str] {
    static ALPHABET: OnceLock<Vec<&'static str>> = OnceLock::new();
    ALPHABET.get_or_init(|| {
        let mut symbols = Vec::new();
        for emoji in emojis::iter() {
            if let Some(variants) = emoji.skin_tones() {
                symbols.extend(variants.map(emojis::Emoji::as_str));
            } else {
                symbols.push(emoji.as_str());
            }
        }
        // The package excludes the nine standalone emoji components included
        // in Unicode 17.0's total: five skin tones and four hair components.
        // https://www.unicode.org/emoji/charts-17.0/emoji-counts.html
        symbols.extend(["🏻", "🏼", "🏽", "🏾", "🏿", "🦰", "🦱", "🦲", "🦳"]);
        symbols.sort_unstable();
        symbols.dedup();
        symbols
    })
}

/// Encode a complete SHA-256 value as 22 space-separated emoji symbols.
///
/// Uses the full Unicode 17.0 dataset from the pinned `emojis` package,
/// including skin-tone variants and standalone components, in sorted UTF-8
/// order. Bytes are interpreted as an unsigned big-endian integer, with
/// leading zero digits preserved.
/// The dataset version, ordering, and width are part of the encoding contract.
/// Emoji symbols may contain multiple Unicode code points.
pub(crate) fn sha256_to_emojis(hash: &[u8; 32]) -> String {
    let symbols = alphabet();
    let radix = symbols.len() as u32;
    let mut value = *hash;
    let mut digits = [0_usize; SYMBOL_COUNT];

    // Long division converts the big-endian 256-bit integer without a bigint
    // dependency. Each intermediate is below 256 * radix and fits in u32.
    for digit in digits.iter_mut().rev() {
        let mut remainder = 0_u32;
        for byte in &mut value {
            let dividend = remainder * 256 + u32::from(*byte);
            *byte = (dividend / radix) as u8;
            remainder = dividend % radix;
        }
        *digit = remainder as usize;
    }

    digits.map(|digit| symbols[digit]).join(" ")
}
// sirno:witness:emoji-fingerprint:end

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn alphabet_contains_all_unicode_17_emojis_without_duplicates() {
        let symbols = alphabet();
        assert_eq!(symbols.len(), 3953);
        assert!(symbols.windows(2).all(|pair| pair[0] < pair[1]));
        for symbol in ["😀", "👍", "👍🏽", "👩‍💻", "🇺🇸", "🫪", "🏽", "🦰"]
        {
            assert!(symbols.contains(&symbol), "missing {symbol}");
        }
    }

    #[test]
    fn zero_and_one_have_stable_fixed_width_encodings() {
        assert_eq!(sha256_to_emojis(&[0; 32]), vec!["#️⃣"; 22].join(" "));
        let mut one = [0; 32];
        one[31] = 1;
        let mut expected = vec!["#️⃣"; 22];
        expected[21] = "*️⃣";
        assert_eq!(sha256_to_emojis(&one), expected.join(" "));
    }

    #[test]
    fn maximum_hash_has_a_stable_encoding() {
        // Independently calculated with Python integer divmod in base 3953.
        assert_eq!(
            sha256_to_emojis(&[u8::MAX; 32]),
            "⏬ 🤽🏻‍♂️ 🧝🏼‍♂️ 👩🏼‍❤️‍👩🏽 🏄🏽 🫵🏾 👨🏼‍🤝‍👨🏿 👩‍🍼 👨🏻‍🏭 🧚‍♂️ 💁🏾 👨‍🦳 🏊🏾‍♀️ 🧑🏾‍❤️‍💋‍🧑🏽 🎑 🥹 🤲🏽 🕢 🧑🏼‍🦳 ⛹🏽‍♀️ 👬🏼 💇🏾"
        );
    }

    fn decode(encoded: &str) -> [u8; 32] {
        let symbols: Vec<_> = encoded.split(' ').collect();
        assert_eq!(symbols.len(), SYMBOL_COUNT);
        let mut bytes = [0_u8; 32];
        for symbol in symbols {
            let mut carry = alphabet().binary_search(&symbol).unwrap() as u32;
            for byte in bytes.iter_mut().rev() {
                let value = u32::from(*byte) * alphabet().len() as u32 + carry;
                *byte = (value % 256) as u8;
                carry = value / 256;
            }
            assert_eq!(carry, 0, "encoded value exceeds SHA-256 width");
        }
        bytes
    }

    #[test]
    fn full_hash_round_trips_including_zero_bytes_and_maximum_value() {
        for hash in [[0; 32], [u8::MAX; 32], std::array::from_fn(|i| i as u8)] {
            assert_eq!(decode(&sha256_to_emojis(&hash)), hash);
        }
    }

    #[test]
    fn the_same_hash_always_has_the_same_encoding() {
        let hash = std::array::from_fn(|i| i as u8);
        let encoded = sha256_to_emojis(&hash);
        assert!(!encoded.is_empty());
        assert_eq!(sha256_to_emojis(&hash), encoded);
    }

    #[test]
    fn every_byte_position_and_value_affects_the_encoding() {
        for position in 0..32 {
            let mut encodings = HashSet::new();
            for byte in 0..=u8::MAX {
                let mut hash = [0; 32];
                hash[position] = byte;
                let encoded = sha256_to_emojis(&hash);
                assert_eq!(decode(&encoded), hash);
                assert!(encodings.insert(encoded));
            }
        }
    }
}
