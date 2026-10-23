use serde_json::Value;

pub(crate) struct Approval {
    pub title: String,
    pub preview: String,
    pub options: Vec<Choice>,
    pub proposal: Option<Proposal>,
}

pub(crate) struct Choice {
    pub id: String,
    pub label: String,
    pub allows: bool,
}

pub(crate) struct Proposal {
    pub title: String,
    pub reason: String,
    pub apply: &'static str,
}

impl Approval {
    pub fn parse(params: &Value) -> Self {
        let tool = &params["toolCall"];
        let title = tool["title"].as_str().unwrap_or("Use a tool");
        let input = &tool["rawInput"];
        Self {
            title: clip(title, 1000),
            preview: if input.is_null() {
                String::new()
            } else {
                super::json_preview(input, 4096)
            },
            options: params["options"]
                .as_array()
                .into_iter()
                .flatten()
                .take(16)
                .filter_map(|option| {
                    let id = option["optionId"].as_str()?;
                    if id.is_empty() || id.len() > 240 {
                        return None;
                    }
                    Some(Choice {
                        id: id.into(),
                        label: clip(option["name"].as_str()?, 240),
                        allows: matches!(
                            option["kind"].as_str(),
                            Some("allow_once" | "allow_always")
                        ),
                    })
                })
                .collect(),
            proposal: Proposal::parse(title, input),
        }
    }
}

impl Proposal {
    fn parse(tool: &str, input: &Value) -> Option<Self> {
        let field = |key| input[key].as_str().map(|value| clip(value, 2000));
        if tool.ends_with("rename_function") {
            let function = field("function")?;
            let name = field("new_name")?;
            return Some(Self {
                title: format!("Rename {function} to {name}?"),
                reason: "This name is saved in your project. The binary stays unchanged.".into(),
                apply: "Rename",
            });
        }
        if tool.ends_with("add_note") {
            return Some(Self {
                title: format!("Save a note on {}?", field("function")?),
                reason: field("note")?,
                apply: "Save note",
            });
        }
        let address = field("address")?;
        if tool.ends_with("add_comment") {
            let comment = field("comment")?;
            return Some(Self {
                title: if comment.is_empty() {
                    format!("Remove the comment at {address}?")
                } else {
                    format!("Comment {address}?")
                },
                apply: if comment.is_empty() {
                    "Remove"
                } else {
                    "Comment"
                },
                reason: comment,
            });
        }
        if tool.ends_with("bookmark") {
            let label = field("label")
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| address.clone());
            return Some(Self {
                title: format!("Bookmark {label}?"),
                reason: format!("Keep {address} in your project’s sidebar."),
                apply: "Bookmark",
            });
        }
        None
    }
}

fn clip(text: &str, limit: usize) -> String {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = text[..end].to_owned();
    if end < text.len() {
        result.push('…');
    }
    result
}
