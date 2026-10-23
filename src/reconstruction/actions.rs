use crate::error::Result;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

pub(super) fn fingerprint(stage: &str, values: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"prism-reconstruction-action-v1\0");
    write_part(&mut hasher, stage);
    for value in values {
        write_part(&mut hasher, value);
    }
    format!("{:x}", hasher.finalize())
}

pub(super) struct Identity<'a> {
    pub model: &'a str,
    pub prompt: &'a str,
    pub adapter: &'a str,
    pub validation: &'a str,
}

pub(super) struct Record<'a> {
    pub generation: i64,
    pub address: &'a str,
    pub stage: &'a str,
    pub fingerprint: &'a str,
    pub state: &'a str,
    pub output: Option<&'a str>,
    pub identity: Identity<'a>,
}

fn write_part(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

pub(super) fn record(db: &Connection, record: &Record<'_>) -> Result<()> {
    db.execute(
        "INSERT INTO actions(generation,address,stage,fingerprint,state,output,updated,model,prompt,adapter,validation)
         VALUES(?1,?2,?3,?4,?5,?6,unixepoch(),?7,?8,?9,?10)
         ON CONFLICT(generation,address,stage) DO UPDATE SET
         fingerprint=excluded.fingerprint,state=excluded.state,output=excluded.output,
         updated=excluded.updated,model=?7,prompt=?8,adapter=?9,validation=?10",
        params![
            record.generation,
            record.address,
            record.stage,
            record.fingerprint,
            record.state,
            record.output.unwrap_or(""),
            record.identity.model,
            record.identity.prompt,
            record.identity.adapter,
            record.identity.validation,
        ],
    )?;
    Ok(())
}
