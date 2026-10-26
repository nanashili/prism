use super::{Database, root};
use crate::error::Result;
use crate::reconstruction::{DatabaseOpen, ValidationCommand};
use rusqlite::params;
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn build_generation(
    project: &str,
    args: &Value,
    cancelled: &AtomicBool,
) -> Result<Value> {
    let mut database = Database::open(&root(project, args)?, DatabaseOpen::Existing)?;
    database.verify_project(project)?;
    let generation = database.generation(args["generation"].as_i64())?;
    let snapshot = database.snapshot(generation, cancelled)?;
    let completeness = completeness(&database, generation)?;
    if args["export_only"].as_bool().unwrap_or(false) {
        return Ok(
            json!({"snapshot":snapshot.path,"build_id":snapshot.id,"built":false,
            "reconstruction_complete":completeness.complete,
            "stale_source":completeness.stale,
            "incomplete_units":completeness.incomplete}),
        );
    }

    let custom_command = args.get("command");
    let result = super::build_commands::run(
        &snapshot.path,
        &snapshot.language,
        custom_command,
        cancelled,
    );
    let succeeded = result.is_ok();
    let output = match result {
        Ok(output) => output,
        Err(error) => error.to_string(),
    };
    save_build_receipt(&database, &snapshot, succeeded, &output)?;
    let validation = super::language::validation_kind(
        &snapshot.language,
        ValidationCommand::from(custom_command.is_some()),
    );
    let command_identity = super::build_commands::identity(&snapshot.language, custom_command)?;
    let revision = snapshot.revision.to_string();
    let identity = database.db.query_row(
        "SELECT analyzer,model,prompt_version,adapter_version,validation_version
         FROM generations WHERE id=?1",
        [generation],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    )?;
    let validation_fingerprint = super::actions::fingerprint(
        "validate",
        &[
            &snapshot.language,
            validation,
            &revision,
            &command_identity,
            &identity.0,
            &identity.1,
            &identity.2,
            &identity.3,
            &identity.4,
        ],
    );
    super::actions::record(
        &database.db,
        &super::actions::Record {
            generation,
            address: "__generation__",
            stage: "validate",
            fingerprint: &validation_fingerprint,
            state: if succeeded { validation } else { "failed" },
            output: None,
            identity: super::actions::Identity {
                model: &identity.1,
                prompt: &identity.2,
                adapter: &identity.3,
                validation: &identity.4,
            },
        },
    )?;
    Ok(
        json!({"snapshot":snapshot.path,"build_id":snapshot.id,"succeeded":succeeded,
        "validation":validation,"built":succeeded && validation == "built" && completeness.complete,
        "reconstruction_complete":completeness.complete,
        "stale_source":completeness.stale,"incomplete_units":completeness.incomplete,
        "output":output,"behavior_verified":false,
        "identity":{"analyzer":identity.0,"model":identity.1,
        "prompt_version":identity.2,"adapter_version":identity.3,
        "validation_version":identity.4}}),
    )
}

struct Completeness {
    incomplete: i64,
    complete: bool,
    stale: bool,
}

fn completeness(database: &Database, generation: i64) -> Result<Completeness> {
    let incomplete: i64 = database.db.query_row(
        "SELECT count(*) FROM units WHERE generation=?1 AND state NOT IN ('generated','reused')",
        [generation],
        |row| row.get(0),
    )?;
    let files: i64 = database.db.query_row(
        "SELECT count(*) FROM files WHERE generation=?1",
        [generation],
        |row| row.get(0),
    )?;
    Ok(Completeness {
        incomplete,
        complete: incomplete == 0,
        stale: incomplete > 0 && files > 0,
    })
}

fn save_build_receipt(
    database: &Database,
    snapshot: &super::snapshot::Snapshot,
    succeeded: bool,
    output: &str,
) -> Result<()> {
    if let Err(error) = database.db.execute(
        "UPDATE builds SET succeeded=?1,output=?2 WHERE id=?3",
        params![succeeded, output, snapshot.id],
    ) {
        return Err(crate::error::failure(format!(
            "Build succeeded={succeeded}: {output}\nCould not save build receipt: {error}. Snapshot: {}",
            snapshot.path.display(),
        )));
    }
    Ok(())
}
