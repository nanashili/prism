use super::{Fingerprint, SignatureScope, Snapshot, Unknown, listing, signatures};
use crate::analysis::Engine;
use crate::error::{Result, failure};
use crate::events::Sink;
use crate::model::{Procedure, Project};
use crate::storage::Store;
use object::{Object, ObjectSection};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

const IMAGE_LIMIT: u64 = 256 * 1024 * 1024;
const SIGNATURE_BUDGET: usize = 64 * 1024 * 1024;

pub(crate) struct SignatureCache {
    pub namespace: String,
    pub budget: u64,
    pub listing_only: bool,
}

pub(crate) struct Capture<'a> {
    pub engine: &'a mut Engine,
    pub store: &'a mut Store,
    pub cancelled: &'a AtomicBool,
    pub sink: &'a Sink,
    pub cache: &'a SignatureCache,
}

#[derive(Default)]
struct Collected {
    functions: Vec<Fingerprint>,
    unknown: Vec<Unknown>,
    assembly_bytes: usize,
}

impl Capture<'_> {
    pub fn run(&mut self, project: Project) -> Result<Snapshot> {
        let bytes = image(&project, self.cancelled)?;
        let segments = self.engine.tool("list_segments", json!({}))?;
        let architecture = segments
            .pointer("/subject/architecture")
            .and_then(serde_json::Value::as_str);
        let object = object::File::parse(selected_image(&bytes, architecture)?)?;
        let architecture = format!(
            "{:?}:{:?}:{:?}",
            object.architecture(),
            object.endianness(),
            object.format()
        );
        let sections = object
            .sections()
            .map(|section| Ok((section.address(), section.data()?)))
            .collect::<Result<Vec<_>>>()?;

        self.engine.index(self.store, &project.target.sha256)?;
        let inventory = inventory(self.store, &project.target.sha256)?;
        let collected = self.collect(&project, &inventory, &sections)?;

        if crate::project::files::hash(&project.target.path, self.cancelled)?
            != project.target.sha256
        {
            return Err(failure(
                "The binary changed during comparison; retry with a stable copy",
            ));
        }
        Ok(Snapshot {
            project,
            architecture,
            functions: collected.functions,
            unknown: collected.unknown,
        })
    }

    fn collect(
        &mut self,
        project: &Project,
        inventory: &[Procedure],
        sections: &[(u64, &[u8])],
    ) -> Result<Collected> {
        let mut collected = Collected::default();
        for (position, function) in inventory.iter().enumerate() {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(failure("Cancelled"));
            }
            self.sink.analysis(
                &format!("Comparing {}", project.target.name),
                u64::try_from(position)?,
                Some(u64::try_from(inventory.len())?),
            )?;
            if function.kind == "External" {
                collected.unknown.push(Unknown {
                    function: function.clone(),
                    reason: "External function has no local body".into(),
                });
                continue;
            }
            if let Some(cached) = self.store.cache(
                &project.target.sha256,
                &function.address,
                &self.cache.namespace,
            )? {
                let mut fingerprint: Fingerprint = serde_json::from_str(&cached)?;
                fingerprint.function = function.clone();
                collected.functions.push(fingerprint);
                continue;
            }
            self.collect_function(project, function, sections, &mut collected)?;
        }
        normalize_callees(&mut collected.functions, inventory);
        Ok(collected)
    }

    fn collect_function(
        &mut self,
        project: &Project,
        function: &Procedure,
        sections: &[(u64, &[u8])],
        collected: &mut Collected,
    ) -> Result<()> {
        let captured = if self.cache.listing_only {
            read_listing(self.engine, function, sections)
        } else {
            read_fingerprint(self.engine, function, sections)
        };
        match captured {
            Ok((fingerprint, size)) => {
                collected.assembly_bytes += size;
                if collected.assembly_bytes > SIGNATURE_BUDGET {
                    return Err(failure(
                        "Comparison assembly exceeds the 64 MiB work budget",
                    ));
                }
                self.store.cache_view(
                    &project.target.sha256,
                    &function.address,
                    &self.cache.namespace,
                    &serde_json::to_string(&fingerprint)?,
                    self.cache.budget,
                )?;
                collected.functions.push(fingerprint);
            }
            Err(error) => {
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(failure("Cancelled"));
                }
                if !self.engine.is_reusable() {
                    return Err(error);
                }
                collected.unknown.push(Unknown {
                    function: function.clone(),
                    reason: error.to_string().chars().take(1024).collect(),
                });
            }
        }
        Ok(())
    }
}

fn inventory(store: &Store, project: &str) -> Result<Vec<Procedure>> {
    let mut functions = Vec::new();
    loop {
        let page = store.functions(project, "", functions.len())?;
        if page.is_empty() {
            return Ok(functions);
        }
        functions.extend(page);
        if functions.len() > 20_000 {
            return Err(failure(
                "Version comparison supports at most 20,000 indexed functions per image",
            ));
        }
    }
}

fn image(project: &Project, cancelled: &AtomicBool) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(&project.target.path)?
        .take(IMAGE_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? > IMAGE_LIMIT {
        return Err(failure(
            "Version comparison supports native images up to 256 MiB",
        ));
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(failure("Cancelled"));
    }
    if signatures::digest(&bytes) != project.target.sha256 {
        return Err(failure("The binary identity changed; reopen the project"));
    }
    Ok(bytes)
}

fn read_fingerprint(
    engine: &mut Engine,
    function: &Procedure,
    sections: &[(u64, &[u8])],
) -> Result<(Fingerprint, usize)> {
    let assembly =
        engine.normalized("procedure_assembly", json!({"procedure":function.address}))?;
    let assembly = assembly
        .as_str()
        .ok_or("The provider did not return complete assembly text")?;
    if assembly.len() > 1024 * 1024 {
        return Err(failure("Function assembly exceeds 1 MiB"));
    }
    let (normalized, mnemonics) = signatures::instructions(assembly)?;

    let mut dossier =
        engine.normalized("analyze_function", json!({"procedure":function.address}))?;
    if let Some(raw) = dossier.pointer_mut("/raw_result/value") {
        dossier = raw.take();
    }
    if dossier["basic_blocks_limited"] == true || dossier["truncated"] == true {
        return Err(failure("The provider returned incomplete control flow"));
    }
    let blocks = signatures::blocks(&dossier)?;
    let callees = if dossier["references_limited"] == true {
        Vec::new()
    } else {
        dossier["callees"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["address"].as_str())
            .map(str::to_owned)
            .collect()
    };
    Ok((
        Fingerprint {
            function: function.clone(),
            bytes: body_digest(&blocks, sections)?,
            normalized,
            structure: signatures::structure(&blocks, &mnemonics),
            callees,
            scope: SignatureScope::RecoveredBlocks,
        },
        assembly.len(),
    ))
}

fn body_digest(blocks: &[signatures::Block], sections: &[(u64, &[u8])]) -> Result<String> {
    let mut digest = Sha256::new();
    for block in blocks {
        let start = block.start;
        let length = block.end.checked_sub(start).ok_or("Invalid block range")?;
        let bytes = sections
            .iter()
            .find_map(|(base, data)| {
                let offset = usize::try_from(start.checked_sub(*base)?).ok()?;
                let size = usize::try_from(length).ok()?;
                data.get(offset..offset.checked_add(size)?)
            })
            .ok_or("Basic block is outside file-backed sections")?;
        digest.update(length.to_le_bytes());
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn read_listing(
    engine: &mut Engine,
    function: &Procedure,
    sections: &[(u64, &[u8])],
) -> Result<(Fingerprint, usize)> {
    let assembly =
        engine.normalized("procedure_assembly", json!({"procedure":function.address}))?;
    let assembly = assembly.as_str().ok_or("Missing assembly text")?;
    if assembly.len() > 1024 * 1024 {
        return Err(failure("Function listing exceeds 1 MiB"));
    }
    let instructions = listing::parse(assembly)?;
    let first = instructions.first().ok_or("Empty listing")?;
    let last = instructions.last().ok_or("Empty listing")?;
    if Some(first.address) != crate::analysis::graph::address_value(&function.address) {
        return Err(failure("Listing does not start at the indexed function"));
    }
    let tail = engine.normalized(
        "inspect_native_instruction",
        json!({"address":format!("0x{:x}",last.address)}),
    )?;
    if tail["status"] != "decoded" {
        return Err(failure("The final instruction could not be decoded"));
    }
    let length = tail["length"]
        .as_u64()
        .filter(|length| (1..=16).contains(length))
        .ok_or("Invalid instruction length")?;
    let end = last
        .address
        .checked_add(length)
        .ok_or("Instruction range overflow")?;
    let (normalized, mnemonics) = signatures::instructions(assembly)?;
    let (structure, callees) = listing::structure(&instructions, &mnemonics);
    Ok((
        Fingerprint {
            function: function.clone(),
            bytes: listing_digest(first.address, end, sections)?,
            normalized,
            structure,
            callees,
            scope: SignatureScope::ListingSpan,
        },
        assembly.len(),
    ))
}

fn listing_digest(start: u64, end: u64, sections: &[(u64, &[u8])]) -> Result<String> {
    if end <= start {
        return Err(failure("Invalid listing span"));
    }

    let mut digest = Sha256::new();
    digest.update((end - start).to_le_bytes());

    let mut cursor = start;
    while cursor < end {
        let section = sections
            .iter()
            .filter_map(|(base, data)| {
                let length = u64::try_from(data.len()).ok()?;
                let section_end = base.checked_add(length)?;
                (cursor >= *base && cursor < section_end).then_some((*base, data))
            })
            .max_by_key(|(base, _)| *base)
            .ok_or("Listing span is outside file-backed sections")?;
        let (base, data) = section;
        let offset = usize::try_from(cursor - base)?;
        let available = (end - cursor).min(u64::try_from(data.len() - offset)?);
        let length = usize::try_from(available)?;
        digest.update(&data[offset..offset + length]);
        cursor = cursor
            .checked_add(available)
            .ok_or("Listing span overflow")?;
    }

    Ok(format!("{:x}", digest.finalize()))
}

fn selected_image<'a>(bytes: &'a [u8], architecture: Option<&str>) -> Result<&'a [u8]> {
    use object::read::macho::{MachOFatFile32, MachOFatFile64};
    match object::FileKind::parse(bytes)? {
        object::FileKind::MachOFat32 => {
            select_slice(MachOFatFile32::parse(bytes)?.arches(), bytes, architecture)
        }
        object::FileKind::MachOFat64 => {
            select_slice(MachOFatFile64::parse(bytes)?.arches(), bytes, architecture)
        }
        _ => Ok(bytes),
    }
}

fn select_slice<'a, T: object::read::macho::FatArch>(
    arches: &[T],
    bytes: &'a [u8],
    architecture: Option<&str>,
) -> Result<&'a [u8]> {
    let selected = match architecture {
        Some("arm64") => object::Architecture::Aarch64,
        Some("x86_64") => object::Architecture::X86_64,
        Some("x86") => object::Architecture::I386,
        Some("arm") => object::Architecture::Arm,
        _ => {
            return Err(failure(
                "The provider must identify its selected architecture for a universal binary; otherwise use an extracted thin slice",
            ));
        }
    };
    let mut matching = arches.iter().filter(|arch| arch.architecture() == selected);
    let arch = matching
        .next()
        .ok_or("Provider architecture is missing from the universal binary")?;
    if matching.next().is_some() {
        return Err(failure(
            "Multiple universal slices share this architecture; select a thin slice explicitly",
        ));
    }
    Ok(arch.data(bytes)?)
}

fn normalize_callees(functions: &mut [Fingerprint], inventory: &[Procedure]) {
    // Providers may spell the same address with different zero padding/prefixes.
    let addresses: std::collections::BTreeMap<_, _> = inventory
        .iter()
        .filter_map(|function| {
            Some((
                crate::analysis::graph::address_value(&function.address)?,
                function.address.as_str(),
            ))
        })
        .collect();
    for item in functions {
        item.callees = item
            .callees
            .iter()
            .filter_map(|address| {
                let number = crate::analysis::graph::address_value(address)?;
                addresses.get(&number).map(|address| (*address).to_owned())
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::listing_digest;

    #[test]
    fn listing_digest_covers_the_complete_file_backed_span() -> crate::error::Result<()> {
        let sections = [(0x100, &[1_u8, 2, 3, 4][..])];
        let complete = listing_digest(0x100, 0x104, &sections)?;
        let changed_tail = [(0x100, &[1_u8, 2, 3, 5][..])];
        let adjacent = [(0x100, &[1_u8, 2][..]), (0x102, &[3, 4][..])];
        assert!(listing_digest(0x100, 0x104, &adjacent).is_ok());

        assert_ne!(complete, listing_digest(0x100, 0x104, &changed_tail)?);
        Ok(())
    }

    #[test]
    fn listing_digest_rejects_unbacked_gaps() {
        let sections = [(0x100, &[1_u8, 2][..]), (0x104, &[3, 4][..])];

        assert!(listing_digest(0x100, 0x106, &sections).is_err());
    }
}
