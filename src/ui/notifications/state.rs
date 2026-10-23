use std::collections::VecDeque;
use std::time::{Duration, Instant};

const HISTORY_LIMIT: usize = 24;
const DUPLICATE_WINDOW: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) enum Level {
    Info,
    Success,
    Warning,
    Error,
}

impl Level {
    pub(super) fn lifetime(self) -> Option<Duration> {
        match self {
            Self::Success => Some(Duration::from_secs(5)),
            Self::Info => Some(Duration::from_secs(6)),
            Self::Warning => Some(Duration::from_secs(10)),
            Self::Error => None,
        }
    }
}

pub(super) struct Notice {
    pub id: u64,
    pub level: Level,
    pub title: String,
    pub body: String,
    pub details: String,
    pub scope: String,
    pub chat: Option<i64>,
    pub updated: Instant,
    pub occurrences: u32,
    pub unread: bool,
    pub toast: bool,
    pub expanded: bool,
    pub copied: Option<Instant>,
    deadline: Option<Instant>,
}

impl Notice {
    pub fn new(level: Level, title: &str, body: &str, details: &str, scope: &str) -> Self {
        let now = Instant::now();
        Self {
            id: 0,
            level,
            title: bounded(title, 160),
            body: bounded(body, 400),
            details: bounded(details, 8192),
            scope: bounded(scope, 160),
            chat: None,
            updated: now,
            occurrences: 1,
            unread: true,
            toast: true,
            expanded: false,
            copied: None,
            deadline: level.lifetime().map(|duration| now + duration),
        }
    }

    fn same_message(&self, other: &Self) -> bool {
        self.level == other.level
            && self.title == other.title
            && self.body == other.body
            && self.details == other.details
            && self.scope == other.scope
            && self.chat == other.chat
    }

    pub fn dismiss(&mut self) {
        self.toast = false;
        self.unread = false;
    }

    pub fn keep_visible(&mut self) {
        self.deadline = self
            .level
            .lifetime()
            .map(|duration| Instant::now() + duration);
    }

    pub fn remaining(&self) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    pub fn copy_text(&self) -> String {
        [&self.title, &self.scope, &self.body, &self.details]
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[derive(Default)]
pub(in crate::ui) struct Center {
    pub(super) entries: VecDeque<Notice>,
    pub(in crate::ui) open: bool,
    next_id: u64,
}

impl Center {
    pub(super) fn push(&mut self, mut notice: Notice) {
        let duplicate = self.entries.iter().position(|entry| {
            entry.updated.elapsed() < DUPLICATE_WINDOW && entry.same_message(&notice)
        });
        if let Some(index) = duplicate {
            if let Some(previous) = self.entries.remove(index) {
                notice.id = previous.id;
                notice.occurrences = previous.occurrences.saturating_add(1);
            }
        } else {
            self.next_id = self.next_id.wrapping_add(1);
            notice.id = self.next_id;
        }
        self.entries.push_front(notice);
        self.entries.truncate(HISTORY_LIMIT);
        if self.open {
            self.mark_read();
        }
    }

    pub(in crate::ui) fn unread(&self) -> usize {
        self.entries.iter().filter(|entry| entry.unread).count()
    }

    pub(in crate::ui) fn active_error(&self) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.level == Level::Error && entry.unread)
            .map(|entry| entry.details.as_str())
    }

    pub(in crate::ui) fn recent_error(&self) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.level == Level::Error)
            .map(|entry| entry.details.as_str())
    }

    pub(super) fn mark_read(&mut self) {
        for entry in &mut self.entries {
            entry.dismiss();
        }
    }

    pub(super) fn expire(&mut self) -> bool {
        let mut expired = false;
        for entry in &mut self.entries {
            if entry.toast
                && entry
                    .remaining()
                    .is_some_and(|remaining| remaining.is_zero())
            {
                entry.toast = false;
                expired = true;
            }
        }
        expired
    }
}

fn bounded(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit - "… [truncated]".len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated]", &text[..end])
}
