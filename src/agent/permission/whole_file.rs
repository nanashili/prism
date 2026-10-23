use serde_json::Value;
use std::path::Path;

pub(super) fn reads(params: &Value, file: Option<&Path>) -> bool {
    let Some(file) = file else {
        return false;
    };
    let title = params["toolCall"]["title"].as_str().unwrap_or_default();
    let input = &params["toolCall"]["rawInput"];
    let nested = input["name"] == "read_bytes";
    let reads_bytes = nested || title.ends_with("read_bytes");
    if reads_bytes {
        let input = if nested { &input["arguments"] } else { input };
        let length = ["length", "size", "count"]
            .iter()
            .find_map(|key| input[*key].as_u64());
        return length.is_some_and(|length| match std::fs::metadata(file) {
            Ok(metadata) => length >= metadata.len(),
            Err(_) => true,
        });
    }
    if title.ends_with("open_binary") {
        return false;
    }
    input.as_object().is_some_and(|fields| {
        fields
            .values()
            .filter_map(Value::as_str)
            .any(|value| Path::new(value) == file)
    })
}
