//! Native retrieval signatures over normalized decompiler syntax and saved CFG topology.
//! These features do not implement BSim's data-flow signatures.

use super::Feature;
use crate::analysis::graph::{Graph, address_value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const SPACE: &str = "rust-ghidra-pseudocode-cfg-v1";

pub(crate) fn features(code: &str, graph: Option<&Graph>) -> Vec<Feature> {
    if code.trim().is_empty() {
        return Vec::new();
    }
    let tokens = syntax_tokens(code);
    let mut counts = BTreeMap::<String, u32>::new();
    for window in tokens.windows(3) {
        *counts
            .entry(format!("syntax:{}", window.join(" ")))
            .or_default() += 1;
    }
    if let Some(graph) = graph {
        add_topology(graph, &mut counts);
    }

    // Coalesce the rare hash collision, rather than storing duplicate feature IDs.
    let mut weights = BTreeMap::<u32, f64>::new();
    for (feature, count) in counts {
        let digest = Sha256::digest(feature.as_bytes());
        let hash = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]);
        *weights.entry(hash).or_default() += 1.0 + f64::from(count).ln();
    }
    weights
        .into_iter()
        .map(|(hash, weight)| Feature { hash, weight })
        .collect()
}

fn add_topology(graph: &Graph, counts: &mut BTreeMap<String, u32>) {
    let ordinals: BTreeMap<_, _> = graph
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| Some((address_value(&block.start)?, index)))
        .collect();
    for (index, block) in graph.blocks.iter().enumerate() {
        *counts
            .entry(format!("cfg:successors:{}", block.successors.len()))
            .or_default() += 1;
        for target in &block.successors {
            let direction = match address_value(target).and_then(|address| ordinals.get(&address)) {
                Some(target) if *target <= index => "back",
                Some(_) => "forward",
                None => "external",
            };
            *counts.entry(format!("cfg:edge:{direction}")).or_default() += 1;
        }
    }
}

fn syntax_tokens(code: &str) -> Vec<String> {
    let mut characters = code.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(character) = characters.next() {
        if tokens.len() >= 32_768 {
            break;
        }
        match character {
            character if character.is_whitespace() => {}
            '/' if characters.peek() == Some(&'/') => {
                for character in characters.by_ref() {
                    if character == '\n' {
                        break;
                    }
                }
            }
            '/' if characters.peek() == Some(&'*') => {
                characters.next();
                while let Some(character) = characters.next() {
                    if character == '*' && characters.peek() == Some(&'/') {
                        characters.next();
                        break;
                    }
                }
            }
            '\'' | '"' => {
                skip_literal(&mut characters, character);
                tokens.push("literal".into());
            }
            character if character.is_alphanumeric() || character == '_' => {
                let mut word = String::from(character);
                while characters
                    .peek()
                    .is_some_and(|next| next.is_alphanumeric() || *next == '_')
                {
                    if let Some(next) = characters.next() {
                        word.push(next);
                    }
                }
                let token = if character.is_ascii_digit() {
                    "number"
                } else {
                    keyword(&word)
                };
                tokens.push(token.into());
            }
            other => tokens.push(other.to_string()),
        }
    }
    tokens
}

fn skip_literal(characters: &mut std::iter::Peekable<std::str::Chars<'_>>, quote: char) {
    while let Some(character) = characters.next() {
        if character == '\\' {
            characters.next();
        } else if character == quote {
            break;
        }
    }
}

fn keyword(word: &str) -> &str {
    match word {
        "if" | "else" | "while" | "for" | "do" | "switch" | "case" | "break" | "continue"
        | "return" | "goto" | "sizeof" => word,
        _ => "identifier",
    }
}
