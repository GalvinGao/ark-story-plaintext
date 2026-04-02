use std::collections::HashMap;

use crate::lexer::{Token, extract_param, tokenize_line};

/// A semantic event produced by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Narrative prose text.
    Narration(String),
    /// Character dialogue with speaker name and spoken text.
    Dialogue { speaker: String, text: String },
    /// Subtitle overlay text.
    Subtitle(String),
    /// Sticker overlay text.
    Sticker(String),
}

/// Strip XML/XHTML tags from text, keeping only inner text content.
///
/// Handles tags like `<i>`, `</i>`, `<color=#000000>`, `</color>`.
pub fn strip_xml_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
        } else if ch == '<' {
            in_tag = true;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Replace literal `\n` escape sequences with actual newline characters.
pub fn expand_escape_newlines(text: &str) -> String {
    text.replace("\\n", "\n")
}

/// Full text processing pipeline for Subtitle/Sticker extracted text.
fn process_display_text(text: &str) -> String {
    let text = strip_xml_tags(text);
    expand_escape_newlines(&text)
}

/// Parse an entire input file's content into a sequence of semantic events.
pub fn parse(input: &str) -> Vec<Event> {
    let mut events = Vec::new();
    let mut multi_stickers: HashMap<String, String> = HashMap::new();

    for line in input.lines() {
        let tokens = tokenize_line(line);
        if tokens.is_empty() {
            continue;
        }

        // Single Text token: plain narration.
        if tokens.len() == 1 {
            if let Token::Text(ref text) = tokens[0] {
                events.push(Event::Narration(strip_xml_tags(text)));
                continue;
            }
        }

        // Process tag-based lines.
        let first_tag = tokens.iter().find(|t| matches!(t, Token::Tag { .. }));
        let trailing_text = tokens.iter().find_map(|t| {
            if let Token::Text(s) = t {
                Some(s.as_str())
            } else {
                None
            }
        });

        if let Some(Token::Tag { name, raw_inner }) = first_tag {
            match name.as_str() {
                "name" => {
                    if let Some(text) = trailing_text {
                        if let Some(speaker) = extract_param(raw_inner, "name") {
                            events.push(Event::Dialogue {
                                speaker,
                                text: strip_xml_tags(text),
                            });
                        }
                    }
                }
                "multiline" => {
                    if let Some(text) = trailing_text {
                        if let Some(speaker) = extract_param(raw_inner, "name") {
                            events.push(Event::Dialogue {
                                speaker,
                                text: strip_xml_tags(text),
                            });
                        }
                    }
                }
                "subtitle" => {
                    if let Some(text) = extract_param(raw_inner, "text") {
                        events.push(Event::Subtitle(process_display_text(&text)));
                    }
                    // Closing subtitle tags (no text param) are skipped.
                }
                "sticker" => {
                    let id = extract_param(raw_inner, "id");
                    let text = extract_param(raw_inner, "text");
                    let is_multi = extract_param(raw_inner, "multi")
                        .is_some_and(|v| v.eq_ignore_ascii_case("true"));

                    match (text, is_multi, id) {
                        (Some(t), true, Some(id)) => {
                            // Multi-sticker: accumulate text by id.
                            multi_stickers
                                .entry(id)
                                .or_default()
                                .push_str(&t);
                        }
                        (Some(t), false, _) => {
                            // Single sticker with text: emit immediately.
                            events.push(Event::Sticker(process_display_text(&t)));
                        }
                        (None, _, Some(ref id)) => {
                            // No text: this is a closing/control sticker.
                            // Flush any accumulated multi-sticker text for this id.
                            if let Some(accumulated) = multi_stickers.remove(id) {
                                events.push(Event::Sticker(process_display_text(&accumulated)));
                            }
                        }
                        _ => {
                            // No text, no id: skip.
                        }
                    }
                }
                _ => {
                    // All other tags: skip entirely.
                    // But if there's trailing text after an unknown tag, treat as narration.
                    // (This handles any unexpected patterns defensively.)
                }
            }
        }
    }

    // Flush any remaining multi-stickers that were never closed.
    // (Sort by id for deterministic output.)
    let mut remaining: Vec<_> = multi_stickers.into_iter().collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, text) in remaining {
        events.push(Event::Sticker(process_display_text(&text)));
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- strip_xml_tags tests ---

    #[test]
    fn strip_italic() {
        assert_eq!(strip_xml_tags("<i>hello</i>"), "hello");
    }

    #[test]
    fn strip_color() {
        assert_eq!(strip_xml_tags("<color=#000000>hi</color>"), "hi");
    }

    #[test]
    fn strip_nested_xml() {
        assert_eq!(strip_xml_tags("<color=#000><i>hi</i></color>"), "hi");
    }

    #[test]
    fn strip_no_tags() {
        assert_eq!(strip_xml_tags("plain text"), "plain text");
    }

    #[test]
    fn strip_mixed_content() {
        assert_eq!(strip_xml_tags("a<i>b</i>c"), "abc");
    }

    #[test]
    fn strip_self_closing() {
        assert_eq!(strip_xml_tags("before<br/>after"), "beforeafter");
    }

    #[test]
    fn strip_preserves_ampersand() {
        assert_eq!(strip_xml_tags("a &amp; b"), "a &amp; b");
    }

    // --- expand_escape_newlines tests ---

    #[test]
    fn expand_single_newline() {
        assert_eq!(expand_escape_newlines(r"line1\nline2"), "line1\nline2");
    }

    #[test]
    fn expand_multiple_newlines() {
        assert_eq!(
            expand_escape_newlines(r"a\n\nb"),
            "a\n\nb"
        );
    }

    #[test]
    fn expand_no_newlines() {
        assert_eq!(expand_escape_newlines("no newlines here"), "no newlines here");
    }

    #[test]
    fn expand_leading_newline() {
        assert_eq!(expand_escape_newlines(r"\n联邦移动监狱"), "\n联邦移动监狱");
    }

    // --- parse tests ---

    #[test]
    fn parse_plain_narration() {
        let events = parse("她坐在树桩上一边哼着歌");
        assert_eq!(events, vec![Event::Narration("她坐在树桩上一边哼着歌".into())]);
    }

    #[test]
    fn parse_name_dialogue() {
        let events = parse(r#"[name="锡人"]呼......熟悉的气味"#);
        assert_eq!(
            events,
            vec![Event::Dialogue {
                speaker: "锡人".into(),
                text: "呼......熟悉的气味".into()
            }]
        );
    }

    #[test]
    fn parse_multiline_dialogue() {
        let events = parse(r#"[multiline(name="缪尔赛思")]......塞雷娅！"#);
        assert_eq!(
            events,
            vec![Event::Dialogue {
                speaker: "缪尔赛思".into(),
                text: "......塞雷娅！".into()
            }]
        );
    }

    #[test]
    fn parse_subtitle() {
        let events = parse(r#"[Subtitle(text="星星带走了她的爱人。", x=300)]"#);
        assert_eq!(events, vec![Event::Subtitle("星星带走了她的爱人。".into())]);
    }

    #[test]
    fn parse_subtitle_with_newline() {
        let events = parse(r#"[Subtitle(text="line1\nline2", x=300)]"#);
        assert_eq!(events, vec![Event::Subtitle("line1\nline2".into())]);
    }

    #[test]
    fn parse_subtitle_closing_tag_skipped() {
        let events = parse("[subtitle]");
        assert_eq!(events, vec![]);
    }

    #[test]
    fn parse_sticker_with_xml() {
        let events = parse(r#"[Sticker(id="st1", text="<i>hello</i>", x=320)]"#);
        assert_eq!(events, vec![Event::Sticker("hello".into())]);
    }

    #[test]
    fn parse_sticker_nested_xml() {
        let events =
            parse(r#"[Sticker(id="st1", text="<color=#000000><i>hi</i></color>", x=320)]"#);
        assert_eq!(events, vec![Event::Sticker("hi".into())]);
    }

    #[test]
    fn parse_sticker_closing_no_text_skipped() {
        let events = parse(r#"[Sticker(id="st1",duration=1,block = false)]"#);
        assert_eq!(events, vec![]);
    }

    #[test]
    fn parse_sticker_closing_no_text_no_id() {
        let events = parse(r#"[Sticker(duration=1)]"#);
        assert_eq!(events, vec![]);
    }

    #[test]
    fn parse_multi_sticker_concatenation() {
        let input = r#"[Sticker(id="st1", multi = true, text="1099年，哥伦比亚，", x=320,y=340, alignment="left", size=24, delay=0.04, width=640,block = true)]
[Sticker(id="st1", multi = true, text="\n联邦移动监狱",block = true)]
[Sticker(id="st1",duration=0.5,block = false)]"#;
        let events = parse(input);
        assert_eq!(
            events,
            vec![Event::Sticker("1099年，哥伦比亚，\n联邦移动监狱".into())]
        );
    }

    #[test]
    fn parse_ignored_tags() {
        let input = "[Blocker(a=1, r=0, g=0, b=0)]\n[charslot]\n[Dialog]\n[Delay(time=1)]";
        let events = parse(input);
        assert_eq!(events, vec![]);
    }

    #[test]
    fn parse_decision_omitted() {
        let events =
            parse(r#"[Decision(options="别偷听了;小心被发现",values="1;2")]"#);
        assert_eq!(events, vec![]);
    }

    #[test]
    fn parse_mixed_sequence() {
        let input = r#"[Blocker(a=1)]
[charslot(slot="m")]
她坐在树桩上。
[name="锡人"]呼......
[Dialog]
[Subtitle(text="星星。", x=300)]"#;
        let events = parse(input);
        assert_eq!(
            events,
            vec![
                Event::Narration("她坐在树桩上。".into()),
                Event::Dialogue {
                    speaker: "锡人".into(),
                    text: "呼......".into()
                },
                Event::Subtitle("星星。".into()),
            ]
        );
    }

    #[test]
    fn parse_consecutive_dialogue() {
        let input = r#"[name="小贾斯汀"]看看您的周围，然后回想一下——
[name="小贾斯汀"]您的上司"#;
        let events = parse(input);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Event::Dialogue { speaker, .. } if speaker == "小贾斯汀"));
        assert!(matches!(&events[1], Event::Dialogue { speaker, .. } if speaker == "小贾斯汀"));
    }

    #[test]
    fn parse_sticker_leading_space_preserved() {
        let events = parse(
            r#"[Sticker(id="st1", text="<color=#000000><i> 那么，晚安</i></color>", x=150)]"#,
        );
        assert_eq!(events, vec![Event::Sticker(" 那么，晚安".into())]);
    }

    #[test]
    fn parse_sticker_poem_with_italic() {
        // Use explicit Unicode escapes for Chinese curly quotes.
        let line = format!(
            "[Sticker(id=\"st1\", text=\"<i>\u{201c}如若此后百年千年，来人漫步于繁星身侧，人们便要赞颂她的名。\u{201d}</i>\", x=320)]"
        );
        let events = parse(&line);
        assert_eq!(
            events,
            vec![Event::Sticker(
                "\u{201c}如若此后百年千年，来人漫步于繁星身侧，人们便要赞颂她的名。\u{201d}".into()
            )]
        );
    }

    #[test]
    fn parse_narration_with_xml_tags_stripped() {
        let events = parse("她说了一句<i>重要的话</i>。");
        assert_eq!(events, vec![Event::Narration("她说了一句重要的话。".into())]);
    }

    #[test]
    fn parse_unflushed_multi_sticker() {
        // Multi-sticker that is never closed — should still be flushed at end.
        let input = r#"[Sticker(id="st1", multi = true, text="orphan text")]"#;
        let events = parse(input);
        assert_eq!(events, vec![Event::Sticker("orphan text".into())]);
    }
}
