use crate::error::Result;
use crate::model::Target;
use crate::project::files::TEXT_LIMIT;
use object::read::macho::FatArch;
use object::{Object, ObjectSection, ObjectSegment};
use std::fmt::Write;
use std::fs::File;

#[derive(Default)]
pub(crate) struct Inventory {
    pub imports: usize,
    pub sections: usize,
    pub format: String,
    pub text: String,
}

pub(crate) fn read(target: &Target) -> Result<Inventory> {
    let reader = object::read::ReadCache::new(File::open(&target.path)?);
    let range = match object::FileKind::parse(&reader)? {
        object::FileKind::MachOFat32 => {
            let fat = object::read::macho::MachOFatFile32::parse(&reader)?;
            Some(preferred_slice(fat.arches())?)
        }
        object::FileKind::MachOFat64 => {
            let fat = object::read::macho::MachOFatFile64::parse(&reader)?;
            Some(preferred_slice(fat.arches())?)
        }
        _ => None,
    };
    match range {
        Some((offset, size)) => inventory(reader.range(offset, size)),
        None => inventory(&reader),
    }
}

pub(crate) fn preferred_slice<A: FatArch>(arches: &[A]) -> Result<(u64, u64)> {
    let host = if cfg!(target_arch = "aarch64") {
        object::Architecture::Aarch64
    } else if cfg!(target_arch = "x86_64") {
        object::Architecture::X86_64
    } else {
        object::Architecture::Unknown
    };
    arches
        .iter()
        .find(|arch| arch.architecture() == host)
        .or_else(|| arches.first())
        .map(FatArch::file_range)
        .ok_or_else(|| crate::error::failure("This universal binary contains no architectures"))
}

pub(crate) fn selection(target: &Target, address: u64) -> Result<serde_json::Value> {
    let reader = object::read::ReadCache::new(File::open(&target.path)?);
    let (base, size) = match object::FileKind::parse(&reader)? {
        object::FileKind::MachOFat32 => {
            let fat = object::read::macho::MachOFatFile32::parse(&reader)?;
            preferred_slice(fat.arches())?
        }
        object::FileKind::MachOFat64 => {
            let fat = object::read::macho::MachOFatFile64::parse(&reader)?;
            preferred_slice(fat.arches())?
        }
        _ => (0, std::fs::metadata(&target.path)?.len()),
    };
    let object = object::File::parse(reader.range(base, size))?;
    for segment in object.segments() {
        let Some(delta) = address
            .checked_sub(segment.address())
            .filter(|delta| *delta < segment.size())
        else {
            continue;
        };
        let (start, length) = segment.file_range();
        let offset = if delta < length {
            Some(
                base.checked_add(start)
                    .and_then(|start| start.checked_add(delta))
                    .ok_or("File offset overflow")?,
            )
        } else {
            None
        };
        let bytes = match offset {
            Some(offset) => crate::project::files::bytes(
                &target.path,
                offset,
                (length - delta).min(4) as usize,
            )?,
            None => Vec::new(),
        };
        let bytes = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(
            serde_json::json!({"offset":offset,"segment":segment.name()?.unwrap_or("Unnamed"),"bytes":bytes}),
        );
    }
    Err("This address is outside the file's mapped segments".into())
}

fn inventory<'data>(reader: impl object::read::ReadRef<'data>) -> Result<Inventory> {
    let object = object::File::parse(reader)?;
    let mut inventory = Inventory {
        format: format!("{:?}", object.format()).replace("MachO", "Mach-O"),
        sections: object
            .sections()
            .filter(|section| section.size() > 0)
            .count(),
        ..Default::default()
    };
    let imports = object.imports()?;
    inventory.imports = imports.len();
    inventory.text.push_str("Imported symbol\tLibrary\n\n");
    for import in imports {
        let name = String::from_utf8_lossy(import.name());
        let library = String::from_utf8_lossy(import.library());
        writeln!(inventory.text, "{name}\t{library}")?;
        if inventory.text.len() >= TEXT_LIMIT {
            inventory.text.push_str("\nDisplay limited to 1 MiB.\n");
            break;
        }
    }
    Ok(inventory)
}
