use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const TOOL: &str = "request_user_input";
pub const MAX_ANSWER_CHARS: usize = 4000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    #[serde(default = "default_title")]
    pub title: String,
    pub question: String,
    #[serde(default)]
    pub options: Vec<String>,
}

impl Question {
    pub fn parse(arguments: Value) -> Result<Self, String> {
        let question: Self =
            serde_json::from_value(arguments).map_err(|error| error.to_string())?;
        question.validate()?;
        Ok(question)
    }

    fn validate(&self) -> Result<(), String> {
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

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Answer {
    Answered {
        answer: String,
        /// One-based choice number. None means the user wrote a custom reply.
        selected_option: Option<usize>,
    },
    Skipped,
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

fn validate_text(label: &str, text: &str, limit: usize) -> Result<(), String> {
    if text.trim().is_empty() || text.chars().count() > limit {
        return Err(format!("{label} must contain 1 to {limit} characters"));
    }
    Ok(())
}
