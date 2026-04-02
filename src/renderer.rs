use crate::parser::Event;

/// Render a sequence of events into the final Markdown output.
///
/// Output uses only two Markdown features:
/// - Double line breaks (`\n\n`) between paragraphs.
/// - Blockquote (`> `) for subtitles and stickers.
pub fn render(events: &[Event]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for event in events {
        match event {
            Event::Narration(text) => {
                parts.push(escape_narration(text));
            }
            Event::Dialogue { speaker, text } => {
                parts.push(format!("{speaker}\u{FF1A}{text}"));
            }
            Event::Subtitle(text) => {
                parts.push(format_blockquote(text));
            }
            Event::Sticker(text) => {
                parts.push(format_blockquote(text));
            }
        }
    }

    parts.join("\n\n")
}

/// Format text as a Markdown blockquote, with `> ` prefix on each line.
fn format_blockquote(text: &str) -> String {
    text.lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Escape narration text that starts with `>` to prevent Markdown blockquote interpretation.
fn escape_narration(text: &str) -> String {
    if text.starts_with('>') {
        format!("\\{text}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_single_narration() {
        let events = vec![Event::Narration("hello".into())];
        assert_eq!(render(&events), "hello");
    }

    #[test]
    fn render_two_narrations() {
        let events = vec![
            Event::Narration("line1".into()),
            Event::Narration("line2".into()),
        ];
        assert_eq!(render(&events), "line1\n\nline2");
    }

    #[test]
    fn render_dialogue() {
        let events = vec![Event::Dialogue {
            speaker: "锡人".into(),
            text: "呼".into(),
        }];
        assert_eq!(render(&events), "锡人\u{FF1A}呼");
    }

    #[test]
    fn render_subtitle_single_line() {
        let events = vec![Event::Subtitle("hello".into())];
        assert_eq!(render(&events), "> hello");
    }

    #[test]
    fn render_subtitle_multi_line() {
        let events = vec![Event::Subtitle("line1\nline2".into())];
        assert_eq!(render(&events), "> line1\n> line2");
    }

    #[test]
    fn render_sticker() {
        let events = vec![Event::Sticker("text".into())];
        assert_eq!(render(&events), "> text");
    }

    #[test]
    fn render_narration_starting_with_gt() {
        let events = vec![Event::Narration("> something".into())];
        assert_eq!(render(&events), "\\> something");
    }

    #[test]
    fn render_mixed_sequence() {
        let events = vec![
            Event::Narration("prose".into()),
            Event::Dialogue {
                speaker: "A".into(),
                text: "hi".into(),
            },
            Event::Subtitle("sub".into()),
        ];
        assert_eq!(render(&events), "prose\n\nA\u{FF1A}hi\n\n> sub");
    }

    #[test]
    fn render_empty_events() {
        let events: Vec<Event> = vec![];
        assert_eq!(render(&events), "");
    }

    #[test]
    fn render_subtitle_with_empty_line() {
        // Text with \n\n produces an empty line in the blockquote.
        let events = vec![Event::Subtitle("a\n\nb".into())];
        assert_eq!(render(&events), "> a\n> \n> b");
    }

    #[test]
    fn render_multi_line_sticker() {
        let events = vec![Event::Sticker("1099年，哥伦比亚，\n联邦移动监狱".into())];
        assert_eq!(
            render(&events),
            "> 1099年，哥伦比亚，\n> 联邦移动监狱"
        );
    }
}
