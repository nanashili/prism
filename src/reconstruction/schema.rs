use crate::error::{Result, failure};
use rusqlite::Connection;

pub(super) fn initialize(db: &Connection) -> Result<()> {
    let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > 4 {
        return Err(failure("This reconstruction needs a newer Prism version"));
    }
    if version == 0 {
        db.execute_batch(
            "BEGIN IMMEDIATE;
         CREATE TABLE IF NOT EXISTS generations(
             id INTEGER PRIMARY KEY, parent INTEGER REFERENCES generations(id),
             project TEXT NOT NULL, language TEXT NOT NULL, instructions TEXT NOT NULL,
             config TEXT NOT NULL, analyzer TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'active',
             created INTEGER NOT NULL DEFAULT(unixepoch()), revision INTEGER NOT NULL DEFAULT 0,
             batch_size INTEGER NOT NULL, workers INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS batches(
             token TEXT PRIMARY KEY, generation INTEGER NOT NULL REFERENCES generations(id),
             worker TEXT NOT NULL, state TEXT NOT NULL, expires INTEGER NOT NULL,
             receipt TEXT, created INTEGER NOT NULL DEFAULT(unixepoch()), finished INTEGER);
         CREATE INDEX IF NOT EXISTS batch_active ON batches(generation,state,expires);
         CREATE TABLE IF NOT EXISTS units(
             generation INTEGER NOT NULL REFERENCES generations(id), address TEXT NOT NULL,
             name TEXT NOT NULL, module TEXT NOT NULL, kind TEXT NOT NULL,
             state TEXT NOT NULL DEFAULT 'pending', lease TEXT REFERENCES batches(token),
             reason TEXT NOT NULL DEFAULT '', evidence TEXT,
             PRIMARY KEY(generation,address));
         CREATE INDEX IF NOT EXISTS unit_queue ON units(generation,state,module,address);
         CREATE INDEX IF NOT EXISTS unit_lease ON units(lease);
         CREATE TABLE IF NOT EXISTS objects(
             digest TEXT PRIMARY KEY, size INTEGER NOT NULL, data BLOB NOT NULL);
         CREATE TABLE IF NOT EXISTS files(
             generation INTEGER NOT NULL REFERENCES generations(id), path TEXT NOT NULL,
             digest TEXT NOT NULL REFERENCES objects(digest), PRIMARY KEY(generation,path));
         CREATE TABLE IF NOT EXISTS links(
             generation INTEGER NOT NULL, address TEXT NOT NULL, path TEXT NOT NULL,
             PRIMARY KEY(generation,address,path),
             FOREIGN KEY(generation,address) REFERENCES units(generation,address),
             FOREIGN KEY(generation,path) REFERENCES files(generation,path));
         CREATE TABLE IF NOT EXISTS builds(
             id INTEGER PRIMARY KEY, generation INTEGER NOT NULL REFERENCES generations(id),
             revision INTEGER NOT NULL, snapshot TEXT NOT NULL, succeeded INTEGER,
             output TEXT NOT NULL DEFAULT '', created INTEGER NOT NULL DEFAULT(unixepoch()));
         PRAGMA user_version=1; COMMIT;",
        )?;
    }
    if version <= 1 {
        migrate_v2(db)?;
    }
    if version <= 2 {
        migrate_v3(db)?;
    }
    if version <= 3 {
        migrate_v4(db)?;
    }
    Ok(())
}

fn migrate_v4(db: &Connection) -> Result<()> {
    db.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE generations ADD COLUMN model TEXT NOT NULL DEFAULT 'default';
         ALTER TABLE generations ADD COLUMN prompt_version TEXT NOT NULL DEFAULT 'v1';
         ALTER TABLE generations ADD COLUMN adapter_version TEXT NOT NULL DEFAULT 'v1';
         ALTER TABLE generations ADD COLUMN validation_version TEXT NOT NULL DEFAULT 'v1';
         PRAGMA user_version=4; COMMIT;",
    )?;
    Ok(())
}

fn migrate_v3(db: &Connection) -> Result<()> {
    db.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE modules ADD COLUMN contract_ready INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE actions ADD COLUMN model TEXT NOT NULL DEFAULT '';
         ALTER TABLE actions ADD COLUMN prompt TEXT NOT NULL DEFAULT '';
         ALTER TABLE actions ADD COLUMN adapter TEXT NOT NULL DEFAULT '';
         ALTER TABLE actions ADD COLUMN validation TEXT NOT NULL DEFAULT '';
         CREATE TABLE IF NOT EXISTS contracts(
             generation INTEGER NOT NULL REFERENCES generations(id),
             module TEXT NOT NULL, interface TEXT NOT NULL DEFAULT '', abi TEXT NOT NULL DEFAULT '',
             interface_hash TEXT NOT NULL DEFAULT '',
             abi_hash TEXT NOT NULL DEFAULT '', unknowns TEXT NOT NULL DEFAULT '',
             ready INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY(generation,module),
             FOREIGN KEY(generation,module) REFERENCES modules(generation,module));
         CREATE INDEX IF NOT EXISTS contract_ready ON contracts(generation,ready);
         PRAGMA user_version=3; COMMIT;",
    )?;
    Ok(())
}

fn migrate_v2(db: &Connection) -> Result<()> {
    db.execute_batch(
        "BEGIN IMMEDIATE;
         CREATE TABLE IF NOT EXISTS modules(
             generation INTEGER NOT NULL REFERENCES generations(id),
             module TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'pending',
             cycle_group TEXT NOT NULL DEFAULT '',
             PRIMARY KEY(generation,module));
         CREATE INDEX IF NOT EXISTS module_queue ON modules(generation,state,module);
         CREATE TABLE IF NOT EXISTS module_dependencies(
             generation INTEGER NOT NULL REFERENCES generations(id),
             module TEXT NOT NULL, depends_on TEXT NOT NULL,
             PRIMARY KEY(generation,module,depends_on),
             FOREIGN KEY(generation,module) REFERENCES modules(generation,module),
             FOREIGN KEY(generation,depends_on) REFERENCES modules(generation,module));
         CREATE TABLE IF NOT EXISTS actions(
             generation INTEGER NOT NULL REFERENCES generations(id),
             address TEXT NOT NULL, stage TEXT NOT NULL, fingerprint TEXT NOT NULL,
             state TEXT NOT NULL, output TEXT NOT NULL DEFAULT '',
             updated INTEGER NOT NULL DEFAULT(unixepoch()),
             PRIMARY KEY(generation,address,stage));
         CREATE INDEX IF NOT EXISTS action_state ON actions(generation,stage,state);
         CREATE TABLE IF NOT EXISTS contracts(
             generation INTEGER NOT NULL REFERENCES generations(id),
             module TEXT NOT NULL, interface TEXT NOT NULL DEFAULT '', abi TEXT NOT NULL DEFAULT '',
             interface_hash TEXT NOT NULL DEFAULT '',
             abi_hash TEXT NOT NULL DEFAULT '', unknowns TEXT NOT NULL DEFAULT '',
             ready INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY(generation,module),
             FOREIGN KEY(generation,module) REFERENCES modules(generation,module));
         CREATE INDEX IF NOT EXISTS contract_ready ON contracts(generation,ready);
         INSERT OR IGNORE INTO modules(generation,module,state)
             SELECT generation,module,
                CASE WHEN sum(state IN ('pending','leased')) > 0 THEN 'pending'
                     WHEN sum(state='blocked') > 0 THEN 'blocked' ELSE 'complete' END
             FROM units GROUP BY generation,module;
         PRAGMA user_version=2; COMMIT;",
    )?;
    Ok(())
}
