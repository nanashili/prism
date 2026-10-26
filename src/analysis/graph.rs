use serde::{Deserialize, Serialize};
use serde_json::Value;

mod evidence;
#[cfg(test)]
mod tests;
pub(crate) use evidence::TraceEvidence;

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct Graph {
    pub blocks: Vec<Block>,
    pub callers: Vec<String>,
    #[serde(default)]
    pub callers_deferred: bool,
    pub callees: Vec<String>,
    #[serde(default)]
    pub dynamic: Vec<TraceEvidence>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Block {
    pub start: String,
    pub end: String,
    pub successors: Vec<String>,
    #[serde(default)]
    pub lines: Vec<String>,
}

impl Graph {
    pub fn from_dossier(value: &Value, assembly: &str) -> crate::error::Result<Self> {
        let blocks = value["basic_blocks"]
            .as_array()
            .ok_or("The provider did not return control-flow blocks")?;
        if blocks.len() > 4096 {
            return Err(crate::error::failure(
                "This function has more than 4096 blocks. Use the assembly view.",
            ));
        }
        let instructions: Vec<_> = assembly
            .lines()
            .filter_map(|line| {
                let (address, instruction) = line.split_once(':')?;
                Some((address_value(address)?, instruction.trim()))
            })
            .collect();
        let ordered = instructions.windows(2).all(|pair| pair[0].0 <= pair[1].0);
        let blocks = blocks
            .iter()
            .map(|value| {
                let start = value["start"]
                    .as_str()
                    .ok_or("A block has no start address")?;
                let end = value["end"].as_str().ok_or("A block has no end address")?;
                let lower = address_value(start).ok_or("Invalid block start address")?;
                let upper = address_value(end).ok_or("Invalid block end address")?;
                let lines = block_lines(&instructions, ordered, lower, upper);
                let successors = value["successors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                Ok(Block {
                    start: start.into(),
                    end: end.into(),
                    successors,
                    lines,
                })
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        let references = |key: &str| {
            value[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["address"].as_str())
                .take(1000)
                .map(str::to_owned)
                .collect()
        };
        Ok(Self {
            blocks,
            callers: references("callers"),
            callers_deferred: value["callers_scanned"] == false,
            callees: references("callees"),
            dynamic: Vec::new(),
        })
    }
}

pub(crate) fn address_value(text: &str) -> Option<u64> {
    u64::from_str_radix(text.trim().strip_prefix("0x").unwrap_or(text.trim()), 16).ok()
}

fn block_lines(instructions: &[(u64, &str)], ordered: bool, lower: u64, upper: u64) -> Vec<String> {
    if ordered {
        let start = instructions.partition_point(|(address, _)| *address < lower);
        instructions[start..]
            .iter()
            .take_while(|(address, _)| *address < upper)
            .take(12)
            .map(|(_, line)| (*line).to_owned())
            .collect()
    } else {
        // External providers may return unsorted assembly; retain its original order.
        instructions
            .iter()
            .filter(|(address, _)| *address >= lower && *address < upper)
            .take(12)
            .map(|(_, line)| (*line).to_owned())
            .collect()
    }
}
