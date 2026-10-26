use crate::models::ParagraphSpan;

pub fn extract_paragraphs(text: &str) -> Vec<ParagraphSpan> {
    let mut paras = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // Skip leading whitespace / blank lines
        while i < n && (chars[i].is_whitespace() || chars[i] == '\n' || chars[i] == '\r') {
            i += 1;
        }
        if i >= n {
            break;
        }

        let start_char = i;
        let mut end_char = i;

        // Scan characters until a double newline (blank line) or EOF
        while i < n {
            if chars[i] == '\n' {
                let mut peek = i + 1;
                while peek < n && (chars[peek] == ' ' || chars[peek] == '\t' || chars[peek] == '\r')
                {
                    peek += 1;
                }
                if peek < n && chars[peek] == '\n' {
                    // Blank line boundary
                    end_char = i;
                    i = peek + 1;
                    break;
                }
            }
            i += 1;
            end_char = i;
        }

        let slice: String = chars[start_char..end_char].iter().collect();
        let trimmed = slice.trim();
        if !trimmed.is_empty() {
            let actual_end = start_char + slice.trim_end().chars().count();
            paras.push(ParagraphSpan {
                index: paras.len(),
                text: trimmed.to_string(),
                start_char,
                end_char: actual_end,
            });
        }
    }

    paras
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_paragraphs_with_offsets() {
        let text = "First paragraph here.\nStill in first paragraph.\n\nSecond paragraph starts here.\n\nThird paragraph.";
        let paras = extract_paragraphs(text);

        assert_eq!(paras.len(), 3);

        assert_eq!(paras[0].index, 0);
        assert!(paras[0].text.starts_with("First paragraph"));
        assert!(paras[0].text.ends_with("Still in first paragraph."));

        assert_eq!(paras[1].index, 1);
        assert_eq!(paras[1].text, "Second paragraph starts here.");

        assert_eq!(paras[2].index, 2);
        assert_eq!(paras[2].text, "Third paragraph.");

        for p in &paras {
            let extracted: String = text
                .chars()
                .skip(p.start_char)
                .take(p.end_char - p.start_char)
                .collect();
            assert_eq!(extracted.trim(), p.text);
        }
    }

    #[test]
    fn test_extract_paragraphs_empty_text() {
        assert!(extract_paragraphs("").is_empty());
        assert!(extract_paragraphs("   \n\n  \n").is_empty());
    }
}
