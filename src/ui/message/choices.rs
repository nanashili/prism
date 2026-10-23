use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

pub(in crate::ui) const MAX_REPLY_BYTES: usize = 32 * 1024;
const MAX_OPTIONS: usize = 8;
const MAX_LABEL_BYTES: usize = 2048;

pub(in crate::ui) struct Choices<'a> {
    pub intro: &'a str,
    pub options: Vec<String>,
}

impl<'a> Choices<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        if text.len() > MAX_REPLY_BYTES {
            return None;
        }

        let mut events = Parser::new_ext(text, Options::ENABLE_TASKLISTS).into_offset_iter();
        let mut depth = 0;
        while let Some((event, range)) = events.next() {
            match event {
                Event::Start(Tag::List(_)) if depth == 0 => {
                    let intro = text[..range.start].trim_end();
                    if text[range.end..].trim().is_empty() && asks_for_choice(intro) {
                        return Some(Self {
                            intro,
                            options: read_options(events)?,
                        });
                    }
                    depth += 1;
                }
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                _ => {}
            }
        }
        None
    }
}

fn asks_for_choice(intro: &str) -> bool {
    let prompt = intro
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    if prompt.len() > 1024 {
        return false;
    }

    let prompt = prompt
        .trim_matches([' ', '\t', '\r', '#', '*', '_', '`'])
        .to_ascii_lowercase();
    let imperative = [
        "choose ",
        "pick ",
        "select ",
        "please choose ",
        "please pick ",
        "please select ",
    ]
    .iter()
    .any(|phrase| prompt.starts_with(phrase));
    if imperative {
        return true;
    }
    if !prompt.ends_with(['?', ':']) {
        return false;
    }

    [
        "which ",
        "you prefer",
        "would you like",
        "do you want",
        "how should",
        "how would",
        "shall we",
        "options:",
        "options are:",
        "approaches:",
        "alternatives:",
    ]
    .iter()
    .any(|phrase| prompt.contains(phrase))
}

fn read_options<'a>(
    events: impl Iterator<Item = (Event<'a>, Range<usize>)>,
) -> Option<Vec<String>> {
    let mut options = Vec::new();
    let mut label = String::new();
    for (event, _) in events {
        match event {
            Event::Start(Tag::Item) if options.len() < MAX_OPTIONS => {}
            Event::End(TagEnd::Item) => {
                let trimmed = label.trim();
                if trimmed.is_empty() {
                    return None;
                }
                options.push(trimmed.to_owned());
                label.clear();
            }
            Event::End(TagEnd::List(_)) => return (options.len() >= 2).then_some(options),
            Event::Text(text) | Event::Code(text) => label.push_str(&text),
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                if !label.ends_with(char::is_whitespace) {
                    label.push(' ');
                }
            }
            Event::Start(Tag::Paragraph | Tag::Emphasis | Tag::Strong | Tag::Link { .. })
            | Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link) => {}
            _ => return None,
        }
        if label.len() > MAX_LABEL_BYTES {
            return None;
        }
    }
    None
}
