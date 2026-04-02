/// A token produced by the lexer from a single input line.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// A bracketed tag. `raw_inner` borrows from the original line
    /// (everything between `[` and `]`).
    Tag { raw_inner: &'a str },
    /// Text content (either standalone plain text or trailing text after tags).
    Text(&'a str),
}

/// Extract the tag name from raw_inner and compare case-insensitively.
/// The tag name is the leading identifier before `(`, `=`, whitespace, or end.
pub fn tag_name_eq(raw_inner: &str, expected: &str) -> bool {
    let bytes = raw_inner.as_bytes();
    let expected_bytes = expected.as_bytes();
    let name_end = bytes
        .iter()
        .position(|&b| b == b'(' || b == b'=' || b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    if name_end != expected_bytes.len() {
        return false;
    }
    bytes[..name_end]
        .iter()
        .zip(expected_bytes)
        .all(|(&a, &b)| a.to_ascii_lowercase() == b)
}

/// Tokenize a single input line into a sequence of `Token`s.
///
/// - Lines not starting with `[` produce a single `Text` token.
/// - Lines starting with `[` are split into `Tag` and optional trailing `Text`.
/// - Inside a tag, quoted strings (`"..."`) are respected so that `]` inside quotes
///   does not prematurely close the tag.
pub fn tokenize_line(line: &str) -> Vec<Token<'_>> {
    let s = line.trim_end();
    if s.is_empty() {
        return vec![];
    }
    let bytes = s.as_bytes();
    if bytes[0] != b'[' {
        return vec![Token::Text(s)];
    }

    let len = bytes.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        if bytes[i] == b'[' {
            i += 1; // skip `[`
            let start = i;
            let mut in_quote = false;
            while i < len {
                let b = bytes[i];
                if in_quote {
                    if b == b'\\' && i + 1 < len && bytes[i + 1] == b'"' {
                        i += 2;
                        continue;
                    }
                    if b == b'"' {
                        in_quote = false;
                    }
                } else {
                    if b == b'"' {
                        in_quote = true;
                    } else if b == b']' {
                        break;
                    }
                }
                i += 1;
            }
            let raw_inner = &s[start..i];
            tokens.push(Token::Tag { raw_inner });
            if i < len {
                i += 1; // skip `]`
            }
        } else {
            let text = &s[i..];
            if !text.is_empty() {
                tokens.push(Token::Text(text));
            }
            break;
        }
    }

    tokens
}

/// Extract a named parameter's value from a tag's raw inner content.
///
/// Handles both quoted values (`key="value"`, `key = "value"`) and
/// unquoted values (`key=123`, `key = true`).
///
/// Returns `None` if the key is not found.
pub fn extract_param<'a>(raw_inner: &'a str, key: &str) -> Option<&'a str> {
    let bytes = raw_inner.as_bytes();
    let key_bytes = key.as_bytes();
    let len = bytes.len();
    let klen = key_bytes.len();

    let mut i = 0;
    while i + klen <= len {
        let at_boundary = i == 0
            || bytes[i - 1] == b'('
            || bytes[i - 1] == b','
            || bytes[i - 1].is_ascii_whitespace();

        if at_boundary && &bytes[i..i + klen] == key_bytes {
            let mut j = i + klen;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < len && bytes[j] == b'=' {
                j += 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len {
                    if bytes[j] == b'"' {
                        j += 1; // skip opening `"`
                        let start = j;
                        while j < len {
                            if bytes[j] == b'\\' && j + 1 < len && bytes[j + 1] == b'"' {
                                j += 2;
                                continue;
                            }
                            if bytes[j] == b'"' {
                                break;
                            }
                            j += 1;
                        }
                        return Some(&raw_inner[start..j]);
                    } else {
                        let start = j;
                        while j < len
                            && bytes[j] != b','
                            && bytes[j] != b')'
                            && !bytes[j].is_ascii_whitespace()
                        {
                            j += 1;
                        }
                        return Some(&raw_inner[start..j]);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- tokenize_line tests ---

    #[test]
    fn plain_text() {
        let tokens = tokenize_line("她坐在树桩上一边哼着歌");
        assert_eq!(tokens, vec![Token::Text("她坐在树桩上一边哼着歌")]);
    }

    #[test]
    fn empty_line() {
        assert_eq!(tokenize_line(""), Vec::<Token>::new());
        assert_eq!(tokenize_line("   "), Vec::<Token>::new());
    }

    #[test]
    fn name_dialogue() {
        let tokens = tokenize_line(r#"[name="锡人"]呼......熟悉的气味"#);
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    raw_inner: r#"name="锡人""#
                },
                Token::Text("呼......熟悉的气味"),
            ]
        );
    }

    #[test]
    fn simple_tag_no_params() {
        let tokens = tokenize_line("[Dialog]");
        assert_eq!(tokens, vec![Token::Tag { raw_inner: "Dialog" }]);
    }

    #[test]
    fn tag_case_insensitive() {
        let tokens = tokenize_line("[HEADER(key=\"test\")]");
        assert!(tag_name_eq(if let Token::Tag { raw_inner } = &tokens[0] { raw_inner } else { "" }, "header"));
    }

    #[test]
    fn tag_with_parens() {
        let tokens = tokenize_line("[Delay(time=1)]");
        assert_eq!(tokens, vec![Token::Tag { raw_inner: "Delay(time=1)" }]);
    }

    #[test]
    fn subtitle_tag() {
        let tokens = tokenize_line(r#"[Subtitle(text="hello world", x=300)]"#);
        assert_eq!(tokens.len(), 1);
        if let Token::Tag { raw_inner } = &tokens[0] {
            assert!(tag_name_eq(raw_inner, "subtitle"));
            assert!(raw_inner.contains("hello world"));
        } else {
            panic!("expected Tag");
        }
    }

    #[test]
    fn sticker_with_html() {
        let line = r#"[Sticker(id="st1", text="<i>hello</i>", x=320)]"#;
        let tokens = tokenize_line(line);
        assert_eq!(tokens.len(), 1);
        if let Token::Tag { raw_inner } = &tokens[0] {
            assert!(tag_name_eq(raw_inner, "sticker"));
            assert!(raw_inner.contains("<i>hello</i>"));
        } else {
            panic!("expected Tag");
        }
    }

    #[test]
    fn quoted_string_containing_bracket() {
        let line = r#"[Tag(text="a]b")]"#;
        let tokens = tokenize_line(line);
        assert_eq!(tokens.len(), 1);
        if let Token::Tag { raw_inner, .. } = &tokens[0] {
            assert!(raw_inner.contains("a]b"));
        } else {
            panic!("expected Tag");
        }
    }

    #[test]
    fn multiple_tags_on_one_line() {
        let tokens = tokenize_line("[Dialog][Blocker(a=1)]");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Tag { raw_inner: "Dialog" });
        assert_eq!(tokens[1], Token::Tag { raw_inner: "Blocker(a=1)" });
    }

    #[test]
    fn trailing_whitespace_only() {
        let tokens = tokenize_line("[Dialog]   ");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Tag { raw_inner: "Dialog" });
    }

    #[test]
    fn multiline_tag_with_trailing_text() {
        let line = r#"[multiline(name="缪尔赛思")]......塞雷娅！"#;
        let tokens = tokenize_line(line);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Tag {
            raw_inner: r#"multiline(name="缪尔赛思")"#,
        });
        assert_eq!(tokens[1], Token::Text("......塞雷娅！"));
    }

    #[test]
    fn chinese_quotes_in_text_param() {
        let line = format!(
            "[Subtitle(text=\"\u{201c}\u{201d}\u{201c}家\u{201d}\")]"
        );
        let tokens = tokenize_line(&line);
        assert_eq!(tokens.len(), 1);
        if let Token::Tag { raw_inner, .. } = &tokens[0] {
            assert!(raw_inner.contains("\u{201c}家\u{201d}"));
        } else {
            panic!("expected Tag");
        }
    }

    // --- extract_param tests ---

    #[test]
    fn extract_text_param() {
        let inner = r#"Subtitle(text="hello world", x=300)"#;
        assert_eq!(extract_param(inner, "text"), Some("hello world"));
    }

    #[test]
    fn extract_with_variable_spacing() {
        let inner = r#"Sticker(id="st1", multi = true, text="abc")"#;
        assert_eq!(extract_param(inner, "multi"), Some("true"));
        assert_eq!(extract_param(inner, "text"), Some("abc"));
        assert_eq!(extract_param(inner, "id"), Some("st1"));
    }

    #[test]
    fn extract_missing_param() {
        let inner = r#"Sticker(id="st1",duration=1)"#;
        assert_eq!(extract_param(inner, "text"), None);
    }

    #[test]
    fn extract_name_from_name_tag() {
        let inner = r#"name="锡人""#;
        assert_eq!(extract_param(inner, "name"), Some("锡人"));
    }

    #[test]
    fn extract_name_from_multiline_tag() {
        let inner = r#"multiline(name="缪尔赛思")"#;
        assert_eq!(extract_param(inner, "name"), Some("缪尔赛思"));
    }

    #[test]
    fn extract_unquoted_numeric() {
        let inner = "Delay(time=1.5)";
        assert_eq!(extract_param(inner, "time"), Some("1.5"));
    }

    #[test]
    fn extract_param_not_substring_match() {
        let inner = r#"Sticker(id="st1", width=700)"#;
        assert_eq!(extract_param(inner, "id"), Some("st1"));
        assert_eq!(extract_param(inner, "width"), Some("700"));
    }

    #[test]
    fn extract_text_with_html() {
        let inner = r#"Sticker(id="st1", text="<i>hello</i>")"#;
        assert_eq!(extract_param(inner, "text"), Some("<i>hello</i>"));
    }

    #[test]
    fn extract_text_with_escaped_newline() {
        let inner = r#"Subtitle(text="line1\nline2")"#;
        assert_eq!(extract_param(inner, "text"), Some(r"line1\nline2"));
    }
}
