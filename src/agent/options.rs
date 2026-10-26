use serde_json::Value;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Pick {
    pub id: String,
    pub value: String,
    #[serde(default)]
    pub modes_api: bool,
}

impl Pick {
    pub(crate) fn matches(&self, menu: &OptionMenu) -> bool {
        self.id == menu.id || (self.id == MODEL_PICK && menu.category == "model")
    }
}

#[derive(Clone)]
pub(crate) struct OptionMenu {
    pub id: String,
    pub name: String,
    pub current: String,
    pub category: String,
    pub choices: Vec<Choice>,
    pub modes_api: bool,
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Choice {
    pub value: String,
    pub name: String,
    #[serde(default, deserialize_with = "description")]
    pub description: String,
}

pub(crate) fn menus(value: &Value) -> Vec<OptionMenu> {
    let mut menus = Vec::new();
    for option in value["configOptions"].as_array().into_iter().flatten() {
        let Some(id) = option["id"].as_str() else {
            continue;
        };
        let mut choices = Vec::new();
        for choice in option["options"].as_array().into_iter().flatten() {
            if let Some(group) = choice["options"].as_array() {
                choices.extend(group.iter().filter_map(parse_choice));
            } else if let Some(choice) = parse_choice(choice) {
                choices.push(choice);
            }
        }
        if choices.is_empty() {
            continue;
        }
        menus.push(OptionMenu {
            id: id.into(),
            name: option["name"].as_str().unwrap_or(id).into(),
            category: option["category"].as_str().unwrap_or("other").into(),
            current: option["currentValue"].as_str().unwrap_or_default().into(),
            choices,
            modes_api: false,
        });
    }
    if let Some(modes) = value["modes"]["availableModes"].as_array()
        && !menus.iter().any(|menu| menu.category == "mode")
    {
        let choices = modes
            .iter()
            .filter_map(|mode| {
                Some(Choice {
                    value: mode["id"].as_str()?.into(),
                    name: mode["name"].as_str()?.into(),
                    description: mode["description"].as_str().unwrap_or_default().into(),
                })
            })
            .collect();
        menus.push(OptionMenu {
            id: "mode".into(),
            name: "Mode".into(),
            category: "mode".into(),
            current: value["modes"]["currentModeId"]
                .as_str()
                .unwrap_or_default()
                .into(),
            choices,
            modes_api: true,
        });
    }
    menus
}

fn parse_choice(value: &Value) -> Option<Choice> {
    Some(Choice {
        value: value["value"].as_str()?.into(),
        name: value["name"].as_str()?.into(),
        description: value["description"].as_str().unwrap_or_default().into(),
    })
}

/// ACP updates replace the option list but preserve the separate modes API state.
pub(crate) fn merge(current: &mut Value, update: Value) {
    if update.get("sessionId").is_some() {
        *current = update;
        return;
    }
    if !current.is_object() {
        *current = serde_json::json!({});
    }
    let Some(fields) = update.as_object() else {
        return;
    };
    for (key, value) in fields {
        if let (Some(destination), Some(fields)) = (current[key].as_object_mut(), value.as_object())
        {
            destination.extend(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        } else {
            current[key] = value.clone();
        }
    }
}

/// A model chosen from a saved catalog is resolved against the live session's option ID.
pub(crate) const MODEL_PICK: &str = "$model";

pub(crate) fn resolve(options: &Value, pick: &Pick) -> Option<Pick> {
    menus(options).into_iter().find_map(|menu| {
        (pick.matches(&menu) && menu.choices.iter().any(|choice| choice.value == pick.value)).then(
            || Pick {
                id: menu.id,
                value: pick.value.clone(),
                modes_api: menu.modes_api,
            },
        )
    })
}

pub(super) fn needs_change(options: &Value, pick: &Pick) -> bool {
    menus(options).iter().any(|menu| {
        menu.id == pick.id
            && menu.modes_api == pick.modes_api
            && menu.current != pick.value
            && menu.choices.iter().any(|choice| choice.value == pick.value)
    })
}

fn description<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(<Option<String> as serde::Deserialize>::deserialize(deserializer)?.unwrap_or_default())
}
