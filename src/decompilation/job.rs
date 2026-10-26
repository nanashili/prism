use super::identity::{Context, Selection};
use crate::error::{Result, failure};
use crate::storage::decompilation::DecompilationError;
use crate::storage::{DecompilationJob, DecompilationJobSpec, Store};
use serde_json::{Value, json};

const MAX_DURABLE_ERROR_DETAILS_BYTES: usize = 64 * 1024;

pub(crate) struct ExportRunner {
    pub(super) job: DecompilationJob,
    _ownership: super::ownership::Ownership,
    #[cfg(feature = "rea-rust-ghidra")]
    pub(crate) batch: Option<super::window::BatchWindow>,
}

impl ExportRunner {
    pub(crate) fn start(store: &Store, context: &Context, args: &Value) -> Result<Value> {
        let selection = Selection::parse(args)?;
        let selection_text = serde_json::to_string(&selection)?;
        let (filter, addresses) = selection.inputs();
        let inventory =
            store.decompilation_inventory_digest(&context.project, filter, addresses)?;
        let id = context.job_id(&selection_text, &inventory)?;
        let created = store.create_decompilation_job_with_items(DecompilationJobSpec {
            id: &id,
            project: &context.project,
            provider: &context.provider,
            selection: &selection_text,
            options: &context.configuration,
            expected_inventory_digest: &inventory,
            filter,
            addresses,
        })?;
        let job = Self::load(store, &id)?;
        Ok(json!({"job": id, "created": created, "status": super::inspection::summary(&job)}))
    }

    pub(crate) fn resume(store: &Store, id: &str, context: &Context) -> Result<Self> {
        let ownership = super::ownership::Ownership::acquire(&context.project)?;
        let mut job = Self::load(store, id)?;
        context.verify(&job)?;
        if job.next_offset < job.total {
            store.update_decompilation_state(id, "running", None)?;
            job.state = "running".into();
            job.error = None;
        }
        Ok(Self {
            job,
            _ownership: ownership,
            #[cfg(feature = "rea-rust-ghidra")]
            batch: None,
        })
    }

    pub(crate) fn next_address(&self, store: &Store) -> Result<Option<String>> {
        if !self.is_running() || self.job.next_offset >= self.job.total {
            return Ok(None);
        }
        let item = store.decompilation_item(&self.job.id, self.job.next_offset)?;
        Ok(Some(
            item.ok_or("The saved function inventory is incomplete")?.0,
        ))
    }

    pub(crate) fn skip_saved(&mut self, store: &Store, address: &str) -> Result<bool> {
        if !store.skip_verified_decompilation_result(
            &self.job.id,
            address,
            self.job.next_offset + 1,
        )? {
            return Ok(false);
        }
        self.refresh(store)?;
        Ok(true)
    }

    pub(crate) fn record_result(
        &mut self,
        store: &Store,
        address: &str,
        result: Result<String>,
    ) -> Result<()> {
        self.record_result_with_details(store, address, result, None)
    }

    pub(crate) fn record_result_with_details(
        &mut self,
        store: &Store,
        address: &str,
        result: Result<String>,
        error_details: Option<Value>,
    ) -> Result<()> {
        let result = result.and_then(|text| {
            if text.len() > 512 * 1024 || serde_json::to_vec(&text)?.len() > 768 * 1024 {
                return Err(failure(
                    "Pseudocode exceeds the bounded function artifact limit",
                ));
            }
            Ok(text)
        });
        let (status, text, error) = match result {
            Ok(text) => ("succeeded", Some(text), None),
            Err(error) => ("failed", None, Some(bounded_error(&error.to_string()))),
        };
        let error_details = match (status, error_details) {
            ("failed", Some(details)) => {
                let serialized = serde_json::to_string(&details)?;
                if serialized.len() > MAX_DURABLE_ERROR_DETAILS_BYTES {
                    return Err(failure(
                        "Typed decompilation error details exceed the durable result limit",
                    ));
                }
                Some(serialized)
            }
            ("failed" | "succeeded", None) => None,
            ("succeeded", Some(_)) => {
                return Err(failure("Successful decompilation included error details"));
            }
            _ => return Err(failure("Invalid durable decompilation result status")),
        };
        let error = error.as_deref().map(|message| DecompilationError {
            message,
            details: error_details.as_deref(),
        });
        store.record_decompilation_result(
            &self.job.id,
            address,
            status,
            text.as_deref(),
            error,
            self.job.next_offset + 1,
        )?;
        self.refresh(store)
    }

    fn refresh(&mut self, store: &Store) -> Result<()> {
        self.job = Self::load(store, &self.job.id)?;
        if self.job.next_offset >= self.job.total {
            self.complete(store)?;
        }
        Ok(())
    }

    pub(crate) fn id(&self) -> &str {
        &self.job.id
    }

    pub(crate) fn matches_context(&self, context: &Context) -> bool {
        context.matches(&self.job)
    }

    pub(crate) fn is_running(&self) -> bool {
        self.job.state == "running"
    }

    pub(crate) fn pause(&mut self, store: &Store, error: Option<&str>) -> Result<()> {
        store.update_decompilation_state(&self.job.id, "paused", error)?;
        self.job.state = "paused".into();
        Ok(())
    }

    pub(crate) fn complete(&mut self, store: &Store) -> Result<()> {
        if self.job.next_offset != self.job.total {
            return Err(failure("The durable job has unprocessed functions"));
        }
        let state = if self.job.failed == 0 {
            "completed"
        } else {
            "completed_with_errors"
        };
        store.update_decompilation_state(&self.job.id, state, None)?;
        self.job.state = state.into();
        Ok(())
    }

    pub(super) fn load(store: &Store, id: &str) -> Result<DecompilationJob> {
        store
            .decompilation_job(id)?
            .ok_or_else(|| failure("The durable decompilation job does not exist"))
    }
}

fn bounded_error(text: &str) -> String {
    let mut end = text.len().min(4096);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
