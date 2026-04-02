use crate::parser::Event;

/// Render a sequence of events into the final Markdown output.
///
/// Output uses only two Markdown features:
/// - Double line breaks (`\n\n`) between paragraphs.
/// - Blockquote (`> `) for subtitles and stickers.
pub fn render(events: &[Event]) -> String {
    let mut out = String::with_capacity(events.len() * 64);

    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        match event {
            Event::Narration(text) => {
                if text.starts_with('>') {
                    out.push('\\');
                }
                out.push_str(text);
            }
            Event::Dialogue { speaker, text } => {
                out.push_str(speaker);
                out.push('\u{FF1A}');
                out.push_str(text);
            }
            Event::Subtitle(text) | Event::Sticker(text) => {
                for (j, line) in text.lines().enumerate() {
                    if j > 0 {
                        out.push('\n');
                    }
                    out.push_str("> ");
                    out.push_str(line);
                }
            }
        }
    }

    out
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
