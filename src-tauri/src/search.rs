use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub fn normalize_text(value: &str) -> String {
    value.nfc().collect()
}

pub fn search_key(value: &str) -> String {
    let mut latin_base = false;
    let mut folded = String::new();
    for character in value.nfkd() {
        if is_combining_mark(character) {
            if !latin_base {
                folded.push(character);
            }
            continue;
        }
        latin_base = is_latin_or_ipa(character);
        folded.extend(character.to_lowercase());
    }
    folded.nfc().collect()
}

fn is_latin_or_ipa(character: char) -> bool {
    matches!(
        character as u32,
        0x0041..=0x007A
            | 0x00C0..=0x024F
            | 0x1D00..=0x1D7F
            | 0x1D80..=0x1DBF
            | 0x1E00..=0x1EFF
            | 0xAB30..=0xAB6F
    )
}

#[cfg(test)]
mod tests {
    use super::{normalize_text, search_key};

    #[test]
    fn folds_diacritics_without_changing_display_text() {
        assert_eq!(search_key("guò"), "guo");
        assert_eq!(normalize_text("guo\u{300}"), "guò");
    }

    #[test]
    fn preserves_non_latin_search_content() {
        assert_eq!(search_key("過"), "過");
        assert_eq!(search_key("བོད"), "བོད");
    }
}
