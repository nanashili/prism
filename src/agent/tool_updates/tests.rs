use super::*;
use serde_json::json;

#[test]
fn bounded_join_matches_existing_preview_at_utf8_and_separator_boundaries() {
    for length in [0, 1, 16380, 16381, 16382, 16383, 16384, 16385, 1024 * 1024] {
        for tail in ["", "x", "é", "🦀"] {
            let large = "a".repeat(length);
            for texts in [
                vec![large.as_str(), tail],
                vec!["", large.as_str(), tail],
                vec![tail, large.as_str()],
            ] {
                let baseline = texts.join("\n\n");
                let expected = (!baseline.is_empty()).then(|| clipped(&baseline));
                assert_eq!(joined_preview(texts.into_iter()), expected);
            }
        }
    }
    assert_eq!(joined_preview(std::iter::empty()), None);
    assert_eq!(
        joined_preview(std::iter::repeat_n("", 17)),
        Some("\n\n".repeat(15))
    );
    assert_eq!(
        joined_preview(std::iter::repeat_n("x", 17)),
        Some(vec!["x"; 16].join("\n\n"))
    );
}

#[test]
fn parsing_preserves_content_selection_and_raw_output_precedence() {
    let mut update = json!({"toolCallId":"id","content":[
        {"content":{"text":"nested"},"text":"fallback"},
        {"image":"ignored"}, {"text":"plain"}
    ]});
    assert_eq!(
        parse(&update).unwrap().output,
        Some("nested\n\nplain".into())
    );
    update["rawOutput"] = Value::Null;
    assert_eq!(parse(&update).unwrap().output, Some("null".into()));
    update["toolCallId"] = Value::Null;
    assert!(parse(&update).is_none());
}
