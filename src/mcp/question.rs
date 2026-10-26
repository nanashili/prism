//! Questions sent through the desktop bridge and the user responses returned to agents.

use crate::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// MCP tool name used to display a question card.
pub const TOOL: &str = "request_user_input";
/// Maximum number of Unicode characters retained in a free-text answer.
pub const MAX_ANSWER_CHARS: usize = 4000;

/// Validated question content displayed in the desktop question card.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    #[serde(default = "default_title")]
    /// Short heading, defaulting to Question when omitted from input.
    pub title: String,
    /// Prompt shown above the response controls.
    pub question: String,
    #[serde(default)]
    /// Ordered choices; an empty list permits a free-text response.
    pub options: Vec<String>,
}

impl Question {
    /// Deserializes and validates a question before it reaches the desktop.
    ///
    /// # Errors
    /// Returns an error for unknown fields, invalid lengths, or duplicate choices.
    pub fn parse(arguments: Value) -> Result<Self, Error> {
        let question: Self = serde_json::from_value(arguments)?;
        question.validate()?;
        Ok(question)
    }

    fn validate(&self) -> Result<(), Error> {
        for (label, text, limit) in [
            ("Title", self.title.as_str(), 80),
            ("Question", self.question.as_str(), 2000),
        ] {
            validate_text(label, text, limit)?;
        }
        if self.options.len() > 8 {
            return Err("Provide at most eight choices".into());
        }
        for (index, option) in self.options.iter().enumerate() {
            validate_text("Choice", option, 2048)?;
            if self.options[..index]
                .iter()
                .any(|previous| previous.trim() == option.trim())
            {
                return Err("Choices must be distinct".into());
            }
        }
        Ok(())
    }
}

/// User response to a question, including explicit skip and dismissal outcomes.
#[derive(Debug, Serialize)]
#[non_exhaustive]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Answer {
    /// The user submitted text or selected a numbered option.
    Answered {
        /// Text returned to the waiting agent.
        answer: String,
        /// One-based choice number. None means the user wrote a custom reply.
        selected_option: Option<usize>,
    },
    /// The user chose to skip the question.
    Skipped,
    /// The card closed without an answer.
    Dismissed,
}

pub(super) fn definition() -> Value {
    super::catalog::tool(
        TOOL,
        "Ask the user a question in Prism's question card and wait for their response. Provide up to eight concise choices, or omit options for free text. The user can always write their own answer, skip or dismiss. Call this whenever you need user input instead of printing a list of choices. Returns answered with answer and a one-based selected_option (null for custom text), skipped, or dismissed. Ask one question at a time per chat. Never treat skip or dismissal as approval.",
        &["question"],
        json!({
            "title":{"type":"string","minLength":1,"maxLength":80,"description":"Short card heading. Defaults to Question."},
            "question":{"type":"string","minLength":1,"maxLength":2000},
            "options":{"type":"array","maxItems":8,"items":{"type":"string","minLength":1,"maxLength":2048}}
        }),
    )
}

fn default_title() -> String {
    "Question".into()
}

fn validate_text(label: &str, text: &str, limit: usize) -> Result<(), Error> {
    if text.trim().is_empty() || text.chars().count() > limit {
        return Err(format!("{label} must contain 1 to {limit} characters").into());
    }
    Ok(())
}
