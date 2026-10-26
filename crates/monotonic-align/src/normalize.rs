use crate::models::SourceToken;
use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

static TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w]+(?:['’][\w]+)*").unwrap());

pub fn normalize_token(text: &str) -> String {
    text.to_lowercase()
        .nfkd()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

pub fn tokenize_source(text: &str) -> Vec<SourceToken> {
    let mut tokens = Vec::new();
    let mut last_byte = 0;
    let mut current_char = 0;

    for m in TOKEN_REGEX.find_iter(text) {
        current_char += text[last_byte..m.start()].chars().count();
        let char_len = m.as_str().chars().count();
        let start_char = current_char;
        let end_char = start_char + char_len;
        current_char = end_char;
        last_byte = m.end();

        let matched_text = m.as_str().to_string();
        let normalized = normalize_token(&matched_text);

        tokens.push(SourceToken {
            text: matched_text,
            normalized,
            start_char,
            end_char,
            start: None,
            end: None,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_token_handles_case_and_curly_apostrophes() {
        assert_eq!(normalize_token("Alice"), "alice");
        assert_eq!(normalize_token("GIRL'S"), "girls");
        assert_eq!(normalize_token("girl’s"), "girls");
        assert_eq!(normalize_token("cook-stove"), "cookstove");
    }

    #[test]
    fn test_normalize_token_preserves_spelling_differences() {
        // Essential invariant: gray vs grey must not normalize to the same string!
        assert_ne!(normalize_token("gray"), normalize_token("grey"));
        assert_eq!(normalize_token("gray"), "gray");
        assert_eq!(normalize_token("grey"), "grey");
    }

    #[test]
    fn test_tokenize_source_tracks_exact_character_offsets() {
        let text = "Alice lived in London with Dinah.";
        let tokens = tokenize_source(text);

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].text, "Alice");
        assert_eq!(tokens[0].normalized, "alice");
        assert_eq!(tokens[0].start_char, 0);
        assert_eq!(tokens[0].end_char, 5);
        assert_eq!(&text[tokens[0].start_char..tokens[0].end_char], "Alice");

        assert_eq!(tokens[5].text, "Dinah");
        assert_eq!(tokens[5].normalized, "dinah");
        assert_eq!(&text[tokens[5].start_char..tokens[5].end_char], "Dinah");
    }

    #[test]
    fn test_tokenize_source_handles_contractions_and_apostrophes() {
        let text = "The farmer’s wife didn't scream.";
        let tokens = tokenize_source(text);

        assert_eq!(tokens[1].text, "farmer’s");
        assert_eq!(tokens[1].normalized, "farmers");

        assert_eq!(tokens[3].text, "didn't");
        assert_eq!(tokens[3].normalized, "didnt");
    }
}
