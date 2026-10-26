//! Conservative matching evidence when a provider exposes listings without function bodies.

use super::signatures;
use crate::analysis::graph::address_value;
use crate::error::{Result, failure};
use std::collections::BTreeMap;

pub(super) struct Instruction<'a> {
    pub address: u64,
    pub code: &'a str,
}

pub(super) fn parse(assembly: &str) -> Result<Vec<Instruction<'_>>> {
    let mut instructions = Vec::new();
    for line in assembly.lines().filter(|line| !line.trim().is_empty()) {
        let (address, code) = line.split_once(':').ok_or("Missing instruction address")?;
        let address = address_value(address).ok_or("Invalid instruction address")?;
        let code = code.split(';').next().unwrap_or_default().trim();
        if code.is_empty() {
            return Err(failure("Empty instruction"));
        }
        instructions.push(Instruction { address, code });
    }
    if instructions.is_empty() || instructions.len() > 65_536 {
        return Err(failure("Expected 1 to 65,536 complete instructions"));
    }
    if instructions
        .windows(2)
        .any(|pair| pair[0].address >= pair[1].address)
    {
        return Err(failure("Instruction listing is not strictly ordered"));
    }
    Ok(instructions)
}

/// Preserve direct branch topology by instruction ordinal. This is a listing
/// signature, not a claim that a CFG or complete function body was recovered.
pub(super) fn structure(
    instructions: &[Instruction<'_>],
    mnemonics: &str,
) -> (String, Vec<String>) {
    let ordinals: BTreeMap<_, _> = instructions
        .iter()
        .enumerate()
        .map(|(ordinal, instruction)| (instruction.address, ordinal))
        .collect();
    let mut edges = Vec::new();
    let mut callees = Vec::new();
    for (ordinal, instruction) in instructions.iter().enumerate() {
        let mnemonic = instruction
            .code
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let call = matches!(mnemonic.as_str(), "bl" | "blx" | "call" | "callq");
        let branch = mnemonic == "b"
            || mnemonic.starts_with("b.")
            || mnemonic.starts_with('j')
            || matches!(
                mnemonic.as_str(),
                "cbz" | "cbnz" | "tbz" | "tbnz" | "br" | "bx"
            );
        if !call && !branch {
            continue;
        }
        let destination = instruction
            .code
            .rsplit(|character: char| character.is_whitespace() || character == ',')
            .find(|token| token.starts_with("0x"))
            .and_then(address_value);
        if branch {
            edges.push((
                ordinal,
                destination.and_then(|address| ordinals.get(&address).copied()),
            ));
        }
        if let Some(destination) = destination
            && (call || matches!(mnemonic.as_str(), "b" | "jmp" | "jmpq"))
        {
            callees.push(format!("0x{destination:x}"));
        }
    }
    (
        signatures::digest(format!("listing-v1:{mnemonics}:{edges:?}").as_bytes()),
        callees,
    )
}
