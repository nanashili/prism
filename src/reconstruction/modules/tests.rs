use super::invalidate_consumers;
use crate::error::Result;
use rusqlite::{Connection, Transaction, params};
use std::time::{Duration, Instant};

fn database() -> Connection {
    let db = Connection::open_in_memory().expect("database: valid fixture operation");
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA cache_size=-2048;")
        .expect("database: valid fixture operation");
    super::super::schema::initialize(&db).expect("database: valid fixture operation");
    for generation in [1, 2] {
        db.execute(
            "INSERT INTO generations(id,project,language,instructions,config,analyzer,batch_size,workers)
             VALUES(?1,'project','rust','','{}','provider',8,1)",
            [generation],
        )
        .expect("database: valid fixture operation");
        db.execute(
            "INSERT INTO batches(token,generation,worker,state,expires)
             VALUES(?1,?2,'worker','active',9999999999)",
            params![format!("lease-{generation}"), generation],
        )
        .expect("database: valid fixture operation");
    }
    db
}

fn add_module(db: &Connection, generation: i64, module: &str, states: &[&str]) {
    db.execute(
        "INSERT INTO modules(generation,module,state) VALUES(?1,?2,'complete')",
        params![generation, module],
    )
    .expect("add module: valid fixture operation");
    for (index, state) in states.iter().enumerate() {
        let address = format!("{module}-{index}");
        db.execute(
            "INSERT INTO units(generation,address,name,module,kind,state,lease)
             VALUES(?1,?2,'function',?3,'function',?4,?5)",
            params![
                generation,
                address,
                module,
                state,
                format!("lease-{generation}")
            ],
        )
        .expect("add module: valid fixture operation");
        for stage in ["analyze", "contract", "emit", "validate"] {
            db.execute(
                "INSERT INTO actions(generation,address,stage,fingerprint,state)
                 VALUES(?1,?2,?3,'hash','complete')",
                params![generation, address, stage],
            )
            .expect("add module: valid fixture operation");
        }
    }
}

fn edge(db: &Connection, generation: i64, module: &str, depends_on: &str) {
    db.execute(
        "INSERT INTO module_dependencies(generation,module,depends_on) VALUES(?1,?2,?3)",
        params![generation, module, depends_on],
    )
    .expect("edge: valid fixture operation");
}

fn fixture() -> Connection {
    let db = database();
    for generation in [1, 2] {
        for module in [
            "a",
            "b",
            "shared",
            "cycle1",
            "cycle2",
            "isolated",
            "\"\\\n雪",
        ] {
            add_module(
                &db,
                generation,
                module,
                &[
                    "generated",
                    "reused",
                    "blocked",
                    "pending",
                    "leased",
                    "complete",
                ],
            );
        }
        for (module, dependency) in [
            ("shared", "a"),
            ("shared", "b"),
            ("cycle1", "shared"),
            ("cycle2", "cycle1"),
            ("cycle1", "cycle2"),
        ] {
            edge(&db, generation, module, dependency);
        }
    }
    db
}

fn snapshot(db: &Connection) -> Vec<String> {
    [
        "SELECT json_array(generation,address,state,lease) FROM units ORDER BY generation,address",
        "SELECT json_array(generation,module,state) FROM modules ORDER BY generation,module",
        "SELECT json_array(generation,address,stage,state) FROM actions ORDER BY generation,address,stage",
    ]
    .into_iter()
    .flat_map(|sql| {
        db.prepare(sql)
            .expect("snapshot: valid fixture operation")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("snapshot: valid fixture operation")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("snapshot: valid fixture operation")
    })
    .collect()
}

#[test]
fn overlapping_roots_and_cycles_preserve_invalidation_semantics() {
    let mut baseline = fixture();
    let mut optimized = fixture();
    let roots = ["a", "b", "a", "\"\\\n雪"].map(str::to_owned);
    let tx = baseline.transaction().expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    reference_invalidate(&tx, 1, &roots).expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    tx.commit().expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    let tx = optimized.transaction().expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    invalidate_consumers(&tx, 1, &roots).expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    tx.commit().expect(
        "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
    );
    assert_eq!(snapshot(&baseline), snapshot(&optimized));

    // Six affected modules, each with generated/reused/blocked units reset.
    let reset: i64 = optimized
        .query_row(
            "SELECT count(*) FROM units WHERE generation=1 AND lease IS NULL",
            [],
            |row| row.get(0),
        )
        .expect(
            "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
        );
    assert_eq!(reset, 18);
    let untouched: i64 = optimized
        .query_row(
            "SELECT count(*) FROM modules WHERE generation=2 AND state='complete'",
            [],
            |row| row.get(0),
        )
        .expect(
            "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
        );
    assert_eq!(untouched, 7);
    let analyze: i64 = optimized
        .query_row(
            "SELECT count(*) FROM actions WHERE generation=1 AND stage='analyze'",
            [],
            |row| row.get(0),
        )
        .expect(
            "overlapping roots and cycles preserve invalidation semantics: valid fixture operation",
        );
    assert_eq!(analyze, 42);
}

#[test]
fn empty_roots_and_rolled_back_changes_leave_the_database_unchanged() {
    let mut db = fixture();
    let original = snapshot(&db);
    let tx = db.transaction().expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    invalidate_consumers(&tx, 1, &[]).expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    tx.commit().expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    assert_eq!(snapshot(&db), original);

    let tx = db.transaction().expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    invalidate_consumers(&tx, 1, &["a".to_owned()]).expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    assert_ne!(snapshot(&tx), original);
    tx.rollback().expect(
        "empty roots and rolled back changes leave the database unchanged: valid fixture operation",
    );
    assert_eq!(snapshot(&db), original);
}

fn chain_fixture(count: usize) -> Connection {
    let mut db = database();
    let tx = db
        .transaction()
        .expect("chain fixture: valid fixture operation");
    for index in 0..count {
        let module = format!("module-{index:04}");
        add_module(&tx, 1, &module, &["generated"]);
        if index > 0 {
            edge(&tx, 1, &module, &format!("module-{:04}", index - 1));
        }
    }
    tx.commit().expect("chain fixture: valid fixture operation");
    db
}

fn sample(
    db: &mut Connection,
    roots: &[String],
    invalidate: fn(&Transaction<'_>, i64, &[String]) -> Result<()>,
) -> Duration {
    let tx = db.transaction().expect("sample: valid fixture operation");
    let start = Instant::now();
    invalidate(&tx, 1, roots).expect("sample: valid fixture operation");
    let elapsed = start.elapsed();
    // Every sample starts from the same populated state; fixture/reset cost is excluded.
    tx.rollback().expect("sample: valid fixture operation");
    elapsed
}

#[test]
#[ignore = "release performance benchmark; run serially with --nocapture"]
fn benchmark_reconstruction_invalidation() {
    for (nodes, changed) in [(128, 1), (128, 32), (512, 32)] {
        let mut baseline = chain_fixture(nodes);
        let mut optimized = chain_fixture(nodes);
        let roots = (0..changed)
            .map(|index| format!("module-{index:04}"))
            .collect::<Vec<_>>();
        // Check equivalence on the benchmark fixture before timing it.
        let old_tx = baseline
            .transaction()
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        let new_tx = optimized
            .transaction()
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        reference_invalidate(&old_tx, 1, &roots)
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        invalidate_consumers(&new_tx, 1, &roots)
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        assert_eq!(snapshot(&old_tx), snapshot(&new_tx));
        old_tx
            .rollback()
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        new_tx
            .rollback()
            .expect("benchmark reconstruction invalidation: valid fixture operation");
        let mut old = Vec::new();
        let mut new = Vec::new();
        for iteration in 0..7 {
            // Alternate order to reduce systematic cache/thermal bias.
            if iteration % 2 == 0 {
                old.push(sample(&mut baseline, &roots, reference_invalidate));
                new.push(sample(&mut optimized, &roots, invalidate_consumers));
            } else {
                new.push(sample(&mut optimized, &roots, invalidate_consumers));
                old.push(sample(&mut baseline, &roots, reference_invalidate));
            }
        }
        old.sort();
        new.sort();
        println!(
            "reconstruction_invalidation nodes={nodes} roots={changed} samples=7 before_ms={:.3} after_ms={:.3} speedup={:.2}x",
            old[3].as_secs_f64() * 1000.0,
            new[3].as_secs_f64() * 1000.0,
            old[3].as_secs_f64() / new[3].as_secs_f64(),
        );
    }
}

// Exact pre-optimization implementation from 4c0040d, retained only for parity
// tests and paired measurements against the production function above.
fn reference_invalidate(tx: &Transaction<'_>, generation: i64, modules: &[String]) -> Result<()> {
    for module in modules {
        tx.execute(
            "UPDATE units SET state='pending',lease=NULL WHERE generation=?1 AND module IN
             (WITH RECURSIVE affected(module) AS (
                SELECT ?2 UNION SELECT d.module FROM module_dependencies d
                JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
             ) SELECT module FROM affected) AND state IN ('generated','reused','blocked')",
            params![generation, module],
        )?;
        tx.execute(
            "UPDATE modules SET state='pending' WHERE generation=?1 AND module IN
             (WITH RECURSIVE affected(module) AS (
                SELECT ?2 UNION SELECT d.module FROM module_dependencies d
                JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
             ) SELECT module FROM affected)",
            params![generation, module],
        )?;
        tx.execute(
            "DELETE FROM actions WHERE generation=?1 AND stage IN ('contract','emit','validate')
             AND address IN (SELECT address FROM units WHERE generation=?1 AND module IN
               (WITH RECURSIVE affected(module) AS (
                  SELECT ?2 UNION SELECT d.module FROM module_dependencies d
                  JOIN affected a ON d.depends_on=a.module WHERE d.generation=?1
               ) SELECT module FROM affected))",
            params![generation, module],
        )?;
    }
    Ok(())
}
