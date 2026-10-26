use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BinaryHeap;

#[cfg(test)]
mod tests;

mod evidence;
#[cfg(test)]
mod evidence_tests;
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
        let mut instructions = Instructions::new(instructions, blocks.len());
        let blocks = blocks
            .iter()
            .map(|value| {
                let start = value["start"]
                    .as_str()
                    .ok_or("A block has no start address")?;
                let end = value["end"].as_str().ok_or("A block has no end address")?;
                let lower = address_value(start).ok_or("Invalid block start address")?;
                let upper = address_value(end).ok_or("Invalid block end address")?;
                let lines = instructions.block_lines(lower, upper);
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

struct Instructions<'a> {
    rows: Vec<(u64, &'a str)>,
    ordered: bool,
    indexable: bool,
    by_address: Option<Vec<usize>>,
}

impl<'a> Instructions<'a> {
    fn new(rows: Vec<(u64, &'a str)>, blocks: usize) -> Self {
        let ordered = rows.windows(2).all(|pair| pair[0].0 <= pair[1].0);
        Self {
            rows,
            ordered,
            // A single block never benefits from sorting the whole assembly first.
            indexable: blocks > 1,
            by_address: None,
        }
    }

    fn block_lines(&mut self, lower: u64, upper: u64) -> Vec<String> {
        if lower >= upper {
            return Vec::new();
        }
        if self.ordered {
            let start = self.rows.partition_point(|(address, _)| *address < lower);
            return self.rows[start..]
                .iter()
                .take_while(|(address, _)| *address < upper)
                .take(12)
                .map(|(_, line)| (*line).to_owned())
                .collect();
        }
        if self.by_address.is_none() && self.indexable {
            // Broad blocks often fill their preview in a short provider-order
            // prefix. Avoid building an index unless a block needs a longer scan.
            let mut first = [0; 12];
            let mut count = 0;
            for (index, (address, _)) in self.rows.iter().take(64).enumerate() {
                if *address >= lower && *address < upper {
                    first[count] = index;
                    count += 1;
                    if count == 12 {
                        break;
                    }
                }
            }
            if count == 12 || self.rows.len() <= 64 {
                return first[..count]
                    .iter()
                    .map(|index| self.rows[*index].1.to_owned())
                    .collect();
            }
            let mut indexes: Vec<_> = (0..self.rows.len()).collect();
            indexes.sort_unstable_by_key(|index| self.rows[*index].0);
            self.by_address = Some(indexes);
        }
        let Some(indexes) = &self.by_address else {
            return self.scan_lines(lower, upper);
        };

        // Only visit this block's address range, then select its first twelve
        // rows in the provider's original order. Keep the selection bounded
        // even for overlapping blocks or many duplicate instruction addresses.
        let start = indexes.partition_point(|index| self.rows[*index].0 < lower);
        let count = indexes[start..].partition_point(|index| self.rows[*index].0 < upper);
        if count > 64 {
            // Broad/overlapping ranges can reach twelve rows quickly in the
            // original order. Do not scan all of their indexed candidates.
            return self.scan_lines(lower, upper);
        }
        let mut first = BinaryHeap::with_capacity(12);
        for &index in &indexes[start..start + count] {
            if first.len() < 12 {
                first.push(index);
            } else if let Some(mut last) = first.peek_mut()
                && index < *last
            {
                *last = index;
            }
        }
        first
            .into_sorted_vec()
            .into_iter()
            .map(|index| self.rows[index].1.to_owned())
            .collect()
    }

    fn scan_lines(&self, lower: u64, upper: u64) -> Vec<String> {
        self.rows
            .iter()
            .filter(|(address, _)| *address >= lower && *address < upper)
            .take(12)
            .map(|(_, line)| (*line).to_owned())
            .collect()
    }
}
