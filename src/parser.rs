use std::collections::HashMap;

use crate::lexer::{extract_param, tag_name_eq};

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
/// Returns the input unchanged (no allocation) if no `<` is present.
pub fn strip_xml_tags(text: &str) -> String {
    // Fast path: no XML tags at all (common for narration/dialogue).
    if !text.as_bytes().contains(&b'<') {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            result.push_str(&text[start..i]);
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

/// Parse a line starting with `[`, returning (raw_inner of first tag, trailing text).
/// Scans bytes directly, respecting quoted strings.
#[inline]
fn scan_tag_line(line: &str) -> (&str, Option<&str>) {
    let bytes = line.as_bytes();
    debug_assert!(bytes[0] == b'[');
    let len = bytes.len();
    let mut i = 1; // skip `[`
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
    let raw_inner = &line[start..i];
    if i < len {
        i += 1; // skip `]`
    }
    // Skip any further tags (e.g. [Dialog][Blocker]) — find trailing text
    while i < len && bytes[i] == b'[' {
        i += 1;
        in_quote = false;
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
        if i < len {
            i += 1;
        }
    }
    let trailing = if i < len {
        let t = line[i..].trim_end();
        if t.is_empty() { None } else { Some(t) }
    } else {
        None
    };
    (raw_inner, trailing)
}

/// Parse an entire input file's content into a sequence of semantic events.
pub fn parse(input: &str) -> Vec<Event> {
    let mut events = Vec::with_capacity(input.len() / 40);
    let mut multi_stickers: HashMap<String, String> = HashMap::new();

    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        let bytes = line.as_bytes();

        // Plain text line (no tag).
        if bytes[0] != b'[' {
            // Fast path: narration lines almost never contain XML tags.
            if bytes.contains(&b'<') {
                events.push(Event::Narration(strip_xml_tags(line)));
            } else {
                events.push(Event::Narration(line.to_string()));
            }
            continue;
        }

        // Fast reject: check first byte after `[` to skip the vast majority of ignored tags.
        // We only care about tags starting with n(ame), m(ultiline), s(ticker/ubtitle).
        if bytes.len() < 2 {
            continue;
        }
        let first = bytes[1].to_ascii_lowercase();
        if first != b'n' && first != b'm' && first != b's' {
            continue;
        }

        // Tag line — inline scan instead of allocating Vec<Token>.
        let (raw_inner, trailing) = scan_tag_line(line);

        if tag_name_eq(raw_inner, "name") || tag_name_eq(raw_inner, "multiline") {
            if let Some(text) = trailing {
                if let Some(speaker) = extract_param(raw_inner, "name") {
                    events.push(Event::Dialogue {
                        speaker: speaker.to_string(),
                        text: strip_xml_tags(text),
                    });
                }
            }
        } else if tag_name_eq(raw_inner, "subtitle") {
            if let Some(text) = extract_param(raw_inner, "text") {
                events.push(Event::Subtitle(process_display_text(text)));
            }
        } else if tag_name_eq(raw_inner, "sticker") {
            let id = extract_param(raw_inner, "id");
            let text = extract_param(raw_inner, "text");
            let is_multi = extract_param(raw_inner, "multi")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));

            match (text, is_multi, id) {
                (Some(t), true, Some(id)) => {
                    multi_stickers
                        .entry(id.to_string())
                        .or_default()
                        .push_str(t);
                }
                (Some(t), false, _) => {
                    events.push(Event::Sticker(process_display_text(t)));
                }
                (None, _, Some(id)) => {
                    if let Some(accumulated) = multi_stickers.remove(id) {
                        events.push(Event::Sticker(process_display_text(&accumulated)));
                    }
                }
                _ => {}
            }
        }
    }

    // Flush any remaining multi-stickers that were never closed.
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
        assert_eq!(expand_escape_newlines(r"a\n\nb"), "a\n\nb");
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
        let events = parse(r#"[Decision(options="别偷听了;小心被发现",values="1;2")]"#);
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
        let input = r#"[Sticker(id="st1", multi = true, text="orphan text")]"#;
        let events = parse(input);
        assert_eq!(events, vec![Event::Sticker("orphan text".into())]);
    }
}
