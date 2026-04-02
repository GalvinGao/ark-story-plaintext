/// A token produced by the lexer from a single input line.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A bracketed tag. `name` is lowercase-normalized for easy matching.
    /// `raw_inner` is everything between `[` and `]`, preserved for parameter extraction.
    Tag { name: String, raw_inner: String },
    /// Text content (either standalone plain text or trailing text after tags).
    Text(String),
}

/// Tokenize a single input line into a sequence of `Token`s.
///
/// - Lines not starting with `[` produce a single `Text` token.
/// - Lines starting with `[` are split into `Tag` and optional trailing `Text`.
/// - Inside a tag, quoted strings (`"..."`) are respected so that `]` inside quotes
///   does not prematurely close the tag.
pub fn tokenize_line(line: &str) -> Vec<Token> {
    let s = line.trim_end();
    if s.is_empty() {
        return vec![];
    }
    if !s.starts_with('[') {
        return vec![Token::Text(s.to_string())];
    }

    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        if chars[i] == '[' {
            // Parse a tag: find the matching `]` respecting quoted strings.
            i += 1; // skip `[`
            let start = i;
            let mut in_quote = false;
            while i < len {
                if in_quote {
                    if chars[i] == '\\' && i + 1 < len && chars[i + 1] == '"' {
                        i += 2; // skip escaped quote
                        continue;
                    }
                    if chars[i] == '"' {
                        in_quote = false;
                    }
                } else {
                    if chars[i] == '"' {
                        in_quote = true;
                    } else if chars[i] == ']' {
                        break;
                    }
                }
                i += 1;
            }
            let raw_inner: String = chars[start..i].iter().collect();
            let name = extract_tag_name(&raw_inner);
            tokens.push(Token::Tag { name, raw_inner });
            if i < len {
                i += 1; // skip `]`
            }
        } else {
            // Remaining text after all tags.
            let text: String = chars[i..].iter().collect();
            if !text.is_empty() {
                tokens.push(Token::Text(text));
            }
            break;
        }
    }

    tokens
}

/// Extract and lowercase the tag name from the raw inner content.
///
/// The tag name is the leading identifier before `(`, `=`, whitespace, or end of string.
fn extract_tag_name(raw_inner: &str) -> String {
    let mut name = String::new();
    for ch in raw_inner.chars() {
        if ch == '(' || ch == '=' || ch.is_whitespace() {
            break;
        }
        name.push(ch);
    }
    name.to_lowercase()
}

/// Extract a named parameter's value from a tag's raw inner content.
///
/// Handles both quoted values (`key="value"`, `key = "value"`) and
/// unquoted values (`key=123`, `key = true`).
///
/// Returns `None` if the key is not found.
pub fn extract_param(raw_inner: &str, key: &str) -> Option<String> {
    // We need to find `key` followed by optional whitespace and `=`.
    // We must ensure `key` is at a word boundary (preceded by start, `(`, `,`, or whitespace).
    let chars: Vec<char> = raw_inner.chars().collect();
    let key_chars: Vec<char> = key.chars().collect();
    let len = chars.len();
    let klen = key_chars.len();

    let mut i = 0;
    while i + klen <= len {
        // Check boundary: must be at start or preceded by `(`, `,`, or whitespace.
        let at_boundary = i == 0
            || chars[i - 1] == '('
            || chars[i - 1] == ','
            || chars[i - 1].is_whitespace();

        if at_boundary && chars[i..i + klen].iter().collect::<String>() == key.to_string() {
            // After the key, skip optional whitespace, then expect `=`.
            let mut j = i + klen;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && chars[j] == '=' {
                j += 1; // skip `=`
                while j < len && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < len {
                    if chars[j] == '"' {
                        // Quoted value: read until closing unescaped `"`.
                        j += 1; // skip opening `"`
                        let mut value = String::new();
                        while j < len {
                            if chars[j] == '\\' && j + 1 < len && chars[j + 1] == '"' {
                                value.push('"');
                                j += 2;
                                continue;
                            }
                            if chars[j] == '"' {
                                break;
                            }
                            value.push(chars[j]);
                            j += 1;
                        }
                        return Some(value);
                    } else {
                        // Unquoted value: read until `,`, `)`, or whitespace.
                        let start = j;
                        while j < len
                            && chars[j] != ','
                            && chars[j] != ')'
                            && !chars[j].is_whitespace()
                        {
                            j += 1;
                        }
                        let value: String = chars[start..j].iter().collect();
                        return Some(value);
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
        assert_eq!(tokens, vec![Token::Text("她坐在树桩上一边哼着歌".into())]);
    }

    #[test]
    fn empty_line() {
        assert_eq!(tokenize_line(""), vec![]);
        assert_eq!(tokenize_line("   "), vec![]);
    }

    #[test]
    fn name_dialogue() {
        let tokens = tokenize_line(r#"[name="锡人"]呼......熟悉的气味"#);
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    name: "name".into(),
                    raw_inner: r#"name="锡人""#.into()
                },
                Token::Text("呼......熟悉的气味".into()),
            ]
        );
    }

    #[test]
    fn simple_tag_no_params() {
        let tokens = tokenize_line("[Dialog]");
        assert_eq!(
            tokens,
            vec![Token::Tag {
                name: "dialog".into(),
                raw_inner: "Dialog".into()
            }]
        );
    }

    #[test]
    fn tag_case_insensitive() {
        let tokens = tokenize_line("[HEADER(key=\"test\")]");
        assert_eq!(tokens[0], Token::Tag {
            name: "header".into(),
            raw_inner: "HEADER(key=\"test\")".into(),
        });
    }

    #[test]
    fn tag_with_parens() {
        let tokens = tokenize_line("[Delay(time=1)]");
        assert_eq!(
            tokens,
            vec![Token::Tag {
                name: "delay".into(),
                raw_inner: "Delay(time=1)".into()
            }]
        );
    }

    #[test]
    fn subtitle_tag() {
        let tokens = tokenize_line(r#"[Subtitle(text="hello world", x=300)]"#);
        assert_eq!(tokens.len(), 1);
        if let Token::Tag { name, raw_inner } = &tokens[0] {
            assert_eq!(name, "subtitle");
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
        if let Token::Tag { name, raw_inner } = &tokens[0] {
            assert_eq!(name, "sticker");
            assert!(raw_inner.contains("<i>hello</i>"));
        } else {
            panic!("expected Tag");
        }
    }

    #[test]
    fn quoted_string_containing_bracket() {
        // Hypothetical: `]` inside quoted string must not end the tag.
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
        assert_eq!(tokens[0], Token::Tag {
            name: "dialog".into(),
            raw_inner: "Dialog".into(),
        });
        assert_eq!(tokens[1], Token::Tag {
            name: "blocker".into(),
            raw_inner: "Blocker(a=1)".into(),
        });
    }

    #[test]
    fn trailing_whitespace_only() {
        let tokens = tokenize_line("[Dialog]   ");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Tag {
            name: "dialog".into(),
            raw_inner: "Dialog".into(),
        });
    }

    #[test]
    fn multiline_tag_with_trailing_text() {
        let line = r#"[multiline(name="缪尔赛思")]......塞雷娅！"#;
        let tokens = tokenize_line(line);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Tag {
            name: "multiline".into(),
            raw_inner: r#"multiline(name="缪尔赛思")"#.into(),
        });
        assert_eq!(tokens[1], Token::Text("......塞雷娅！".into()));
    }

    #[test]
    fn chinese_quotes_in_text_param() {
        // Chinese quotes \u{201c} \u{201d} are NOT ASCII " — they don't affect parsing.
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
        assert_eq!(extract_param(inner, "text"), Some("hello world".into()));
    }

    #[test]
    fn extract_with_variable_spacing() {
        let inner = r#"Sticker(id="st1", multi = true, text="abc")"#;
        assert_eq!(extract_param(inner, "multi"), Some("true".into()));
        assert_eq!(extract_param(inner, "text"), Some("abc".into()));
        assert_eq!(extract_param(inner, "id"), Some("st1".into()));
    }

    #[test]
    fn extract_missing_param() {
        let inner = r#"Sticker(id="st1",duration=1)"#;
        assert_eq!(extract_param(inner, "text"), None);
    }

    #[test]
    fn extract_name_from_name_tag() {
        let inner = r#"name="锡人""#;
        assert_eq!(extract_param(inner, "name"), Some("锡人".into()));
    }

    #[test]
    fn extract_name_from_multiline_tag() {
        let inner = r#"multiline(name="缪尔赛思")"#;
        assert_eq!(extract_param(inner, "name"), Some("缪尔赛思".into()));
    }

    #[test]
    fn extract_unquoted_numeric() {
        let inner = "Delay(time=1.5)";
        assert_eq!(extract_param(inner, "time"), Some("1.5".into()));
    }

    #[test]
    fn extract_param_not_substring_match() {
        // "id" should not match "width"
        let inner = r#"Sticker(id="st1", width=700)"#;
        assert_eq!(extract_param(inner, "id"), Some("st1".into()));
        assert_eq!(extract_param(inner, "width"), Some("700".into()));
    }

    #[test]
    fn extract_text_with_html() {
        let inner = r#"Sticker(id="st1", text="<i>hello</i>")"#;
        assert_eq!(extract_param(inner, "text"), Some("<i>hello</i>".into()));
    }

    #[test]
    fn extract_text_with_escaped_newline() {
        let inner = r#"Subtitle(text="line1\nline2")"#;
        assert_eq!(extract_param(inner, "text"), Some(r"line1\nline2".into()));
    }
}
