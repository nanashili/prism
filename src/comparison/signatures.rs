use crate::analysis::graph::address_value;
use crate::error::{Result, failure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Keep registers and operand punctuation; remove comments, labels and numeric values.
/// Masking constants is deliberately reported as heuristic evidence, never exact behavior.
pub(super) fn instructions(assembly: &str) -> Result<(String, String)> {
    let mut normalized = String::new();
    let mut mnemonics = String::new();
    for line in assembly.lines().filter(|line| !line.trim().is_empty()) {
        let (address, code) = line
            .split_once(':')
            .ok_or("Assembly has no instruction address")?;
        address_value(address).ok_or("Assembly has an invalid instruction address")?;
        let code = code.split(';').next().unwrap_or_default().trim();
        let mnemonic = code.split_whitespace().next().ok_or("Empty instruction")?;
        mnemonics.push_str(&mnemonic.to_ascii_lowercase());
        mnemonics.push('\n');
        for token in code.split_inclusive(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_'
        }) {
            let word = token.trim_end_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '_'
            });
            let suffix = &token[word.len()..];
            if word.starts_with(|character: char| character.is_ascii_digit()) {
                normalized.push('#');
            } else {
                normalized.push_str(&word.to_ascii_lowercase());
            }
            for character in suffix.chars() {
                if character.is_whitespace() {
                    if !normalized.ends_with(' ') {
                        normalized.push(' ');
                    }
                } else {
                    normalized.push(character);
                }
            }
        }
        normalized.push('\n');
    }
    if normalized.is_empty() {
        return Err(failure("The provider returned no instructions"));
    }
    Ok((digest(normalized.as_bytes()), digest(mnemonics.as_bytes())))
}

pub(super) struct Block {
    pub start: u64,
    pub end: u64,
    pub successors: Vec<u64>,
}

pub(super) fn blocks(dossier: &Value) -> Result<Vec<Block>> {
    let rows = dossier["basic_blocks"]
        .as_array()
        .ok_or("Missing basic blocks")?;
    if rows.is_empty() || rows.len() > 4096 {
        return Err(failure("Expected between 1 and 4096 complete basic blocks"));
    }
    let mut blocks = Vec::with_capacity(rows.len());
    for row in rows {
        let start = row["start"]
            .as_str()
            .and_then(address_value)
            .ok_or("Invalid block start")?;
        let end = row["end"]
            .as_str()
            .and_then(address_value)
            .ok_or("Invalid block end")?;
        if end <= start {
            return Err(failure("Invalid block extent"));
        }
        let successors = row["successors"]
            .as_array()
            .ok_or("Missing successors")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .and_then(address_value)
                    .ok_or_else(|| failure("Invalid successor"))
            })
            .collect::<Result<Vec<_>>>()?;
        blocks.push(Block {
            start,
            end,
            successors,
        });
    }
    blocks.sort_by_key(|block| block.start);
    if blocks.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err(failure("Overlapping basic blocks"));
    }
    Ok(blocks)
}

pub(super) fn structure(blocks: &[Block], mnemonics: &str) -> String {
    let ordinals: BTreeMap<_, _> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.start, index))
        .collect();
    let edges: Vec<Vec<Option<usize>>> = blocks
        .iter()
        .map(|block| {
            let mut edges: Vec<_> = block
                .successors
                .iter()
                .map(|address| ordinals.get(address).copied())
                .collect();
            edges.sort();
            edges
        })
        .collect();
    digest(format!("{mnemonics}:{edges:?}").as_bytes())
}
