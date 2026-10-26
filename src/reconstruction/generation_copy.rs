use crate::error::Result;
use rusqlite::{Connection, params};

pub(super) fn copy(
    db: &Connection,
    parent: i64,
    id: i64,
    reuse_source: SourceReuse,
    reuse_evidence: EvidenceReuse,
) -> Result<()> {
    let reuse_source = reuse_source == SourceReuse::Reuse;
    let reuse_evidence = reuse_evidence == EvidenceReuse::Reuse;

    db.execute(
        "INSERT INTO units(generation,address,name,module,kind,state,evidence)
         SELECT ?1,address,name,module,kind,
            CASE WHEN ?3 AND state IN ('generated','reused') THEN 'reused' ELSE 'pending' END,
            CASE WHEN ?4 THEN evidence ELSE NULL END
         FROM units WHERE generation=?2",
        params![id, parent, reuse_source, reuse_evidence],
    )?;
    // Keep prior files available for merge/review even when the new identity
    // invalidates source reuse. The child units remain pending in that case.
    copy_sources(db, parent, id)?;
    copy_modules(db, parent, id, reuse_source)?;
    copy_actions(db, parent, id, reuse_source, reuse_evidence)?;
    Ok(())
}

fn copy_sources(db: &Connection, parent: i64, id: i64) -> Result<()> {
    db.execute(
        "INSERT INTO files SELECT ?1,path,digest FROM files WHERE generation=?2",
        params![id, parent],
    )?;
    db.execute(
        "INSERT INTO links SELECT ?1,address,path FROM links WHERE generation=?2",
        params![id, parent],
    )?;
    Ok(())
}

fn copy_modules(db: &Connection, parent: i64, id: i64, reuse_source: bool) -> Result<()> {
    db.execute(
        "INSERT OR IGNORE INTO modules(generation,module,state,cycle_group,contract_ready)
         SELECT ?1,module,'pending',max(cycle_group),CASE WHEN ?3 THEN min(contract_ready) ELSE 0 END
         FROM modules WHERE generation=?2 GROUP BY module",
        params![id, parent, reuse_source],
    )?;
    db.execute(
        "UPDATE modules SET state=CASE
             WHEN EXISTS(SELECT 1 FROM units WHERE generation=?1 AND module=modules.module
                         AND state IN ('pending','leased')) THEN 'pending'
             WHEN EXISTS(SELECT 1 FROM units WHERE generation=?1 AND module=modules.module
                         AND state='blocked') THEN 'blocked'
             ELSE 'complete' END
         WHERE generation=?1",
        [id],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO module_dependencies(generation,module,depends_on)
         SELECT ?1,module,depends_on FROM module_dependencies WHERE generation=?2",
        params![id, parent],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO contracts(generation,module,interface,abi,interface_hash,abi_hash,unknowns,ready)
         SELECT ?1,module,interface,abi,interface_hash,abi_hash,unknowns,
                CASE WHEN ?3 THEN ready ELSE 0 END
         FROM contracts WHERE generation=?2",
        params![id, parent, reuse_source],
    )?;
    Ok(())
}

fn copy_actions(
    db: &Connection,
    parent: i64,
    id: i64,
    reuse_source: bool,
    reuse_evidence: bool,
) -> Result<()> {
    if reuse_evidence {
        copy_stage(db, parent, id, "evidence")?;
    }
    if reuse_source {
        copy_stage(db, parent, id, "contract")?;
        copy_stage(db, parent, id, "emit")?;
    }
    Ok(())
}

fn copy_stage(db: &Connection, parent: i64, id: i64, stage: &str) -> Result<()> {
    db.execute(
        "INSERT INTO actions(generation,address,stage,fingerprint,state,output,updated,model,prompt,adapter,validation)
         SELECT ?1,address,stage,fingerprint,state,output,updated,model,prompt,adapter,validation
         FROM actions WHERE generation=?2 AND stage=?3",
        params![id, parent, stage],
    )?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceReuse {
    Reuse,
    Invalidate,
}

impl From<bool> for SourceReuse {
    fn from(value: bool) -> Self {
        if value { Self::Reuse } else { Self::Invalidate }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvidenceReuse {
    Reuse,
    Invalidate,
}

impl From<bool> for EvidenceReuse {
    fn from(value: bool) -> Self {
        if value { Self::Reuse } else { Self::Invalidate }
    }
}
