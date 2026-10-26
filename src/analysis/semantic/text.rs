use super::Document;
use crate::error::{Result, failure};
use std::collections::BTreeSet;

#[derive(Debug)]
pub(crate) struct Query {
    pub terms: BTreeSet<String>,
}

impl Query {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        if value.len() > 4096 {
            return Err(failure("A behavior query exceeds 4096 bytes"));
        }
        let mut terms = terms(value);
        let lower = value.to_lowercase();
        if lower.contains("object pool")
            || lower.contains("free list")
            || lower.contains("freelist")
        {
            terms.insert("objectpool".into());
        }
        if terms.contains("header") && (terms.contains("parse") || terms.contains("binary")) {
            terms.insert("binaryheader".into());
        }
        if terms.is_empty() || terms.len() > 32 {
            return Err(failure("Use a behavior query with 1–32 meaningful terms"));
        }
        Ok(Self { terms })
    }

    pub(crate) fn fts(&self) -> String {
        // Terms contain only alphanumerics. User text never becomes FTS syntax.
        self.terms
            .iter()
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<_>>()
            .join(" OR ")
    }
}

pub(super) fn terms(value: &str) -> BTreeSet<String> {
    let mut separated = String::with_capacity(value.len());
    let mut previous_lower = false;
    for character in value.chars() {
        if previous_lower && character.is_uppercase() {
            separated.push(' ');
        }
        separated.extend(character.to_lowercase());
        previous_lower = character.is_lowercase();
    }
    separated
        .split(|character: char| !character.is_alphanumeric())
        .filter_map(canonical)
        .map(str::to_owned)
        .collect()
}

fn canonical(term: &str) -> Option<&str> {
    Some(match term {
        "a" | "an" | "and" | "are" | "as" | "at" | "by" | "code" | "does" | "find" | "for"
        | "from" | "function" | "functions" | "implements" | "in" | "is" | "it" | "of" | "or"
        | "that" | "the" | "these" | "this" | "to" | "using" | "with" => return None,
        "parse" | "parses" | "parsing" | "parser" | "decode" | "decodes" | "decoding"
        | "deserialize" | "deserializes" | "deserialization" => "parse",
        "headers" => "header",
        "objects" => "object",
        "allocation" | "allocate" | "allocates" | "allocator" | "malloc" | "calloc" => "alloc",
        "recycle" | "recycles" | "recycling" | "reuse" | "reuses" => "reuse",
        "freelist" => "freelist",
        "verify" | "verifies" | "validate" | "validates" | "validation" | "check" | "checks" => {
            "check"
        }
        "reading" | "reads" | "fread" | "read" => "read",
        "memcpy" | "memmove" | "copying" | "copies" => "copy",
        "memcmp" | "strcmp" | "compare" | "compares" => "compare",
        "ntohl" | "ntohs" | "bswap" | "endian" | "endianness" => "endian",
        "loop" | "loops" | "while" | "forloop" => "loop",
        "" => return None,
        other => other,
    })
}

pub(super) fn behaviors(document: &Document) -> Vec<&'static str> {
    // Names and callers alone cannot establish behavior in the function body.
    let body = format!(
        "{} {} {}",
        body(&document.decompiler),
        document.imports.join(" "),
        document.summary
    );
    let words = terms(&body);
    let constants = !document.constants.is_empty();
    let mut behaviors = Vec::new();
    let reads_data = words.contains("read") || words.contains("copy") || words.contains("parse");
    let checks_format = words.contains("header")
        || words.contains("magic")
        || words.contains("endian")
        || (constants && words.contains("compare"));
    if reads_data && checks_format {
        behaviors.push("binaryheader");
    }

    let free_list =
        words.contains("freelist") || (words.contains("free") && words.contains("list"));
    let recycles = words.contains("reuse") || words.contains("alloc") || words.contains("next");
    if free_list && recycles {
        behaviors.push("objectpool");
    }
    behaviors
}

/// Keep a function's declaration from turning its name into body evidence.
pub(super) fn body(code: &str) -> &str {
    code.split_once('{').map_or(code, |(_, body)| body)
}

fn behavior_reason(behavior: &str) -> &'static str {
    match behavior {
        "binaryheader" => {
            "Inference: data reads/copies or parsing occur with header, magic, endian, or constant-comparison evidence. Inspect the body to confirm."
        }
        "objectpool" => {
            "Inference: a free list occurs with allocation, reuse, or next-link evidence. Inspect ownership and recycling to confirm."
        }
        _ => "Inferred behavior; inspect the supporting evidence.",
    }
}

pub(super) fn excerpt(value: &str, query: &BTreeSet<String>) -> String {
    let line = value
        .lines()
        .find(|line| !terms(line).is_disjoint(query))
        .unwrap_or(value);
    line.chars().take(240).collect()
}

/// Keep the heuristic and its observed inputs together in the explanation.
pub(super) fn behavior_evidence(document: &Document, behavior: &str) -> String {
    let anchors = match behavior {
        "binaryheader" => "read copy parse header magic endian compare",
        "objectpool" => "free freelist list alloc reuse next",
        _ => "",
    };
    let anchors = terms(anchors);
    let lines = body(&document.decompiler)
        .lines()
        .filter(|line| !terms(line).is_disjoint(&anchors))
        .take(3)
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    let code: String = lines.chars().take(240).collect();
    let calls: String = document.imports.join(", ").chars().take(120).collect();
    let constants: String = document.constants.join(", ").chars().take(120).collect();
    format!(
        "{} Calls: {calls}; constants: {constants}; body: {code}",
        behavior_reason(behavior)
    )
}
