use crate::error::{Result, failure};
use crate::settings::Settings;
use crate::storage::DecompilationJob;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Selection {
    All { filter: String },
    Addresses { addresses: Vec<String> },
}

impl Selection {
    pub(super) fn parse(args: &Value) -> Result<Self> {
        if let Some(values) = args.get("addresses") {
            if args.get("filter").is_some() {
                return Err(failure("Choose addresses or a filter, not both"));
            }
            let values = values.as_array().ok_or("Addresses must be an array")?;
            prism::mcp::validate_durable_address_count(values.len()).map_err(failure)?;
            let mut unique = HashSet::with_capacity(values.len());
            let mut addresses = Vec::with_capacity(values.len());
            for value in values {
                let text = value.as_str().ok_or("Every address must be text")?;
                let number = crate::analysis::graph::address_value(text)
                    .ok_or("Every address must be a valid function address")?;
                let address = format!("0x{number:x}");
                if !unique.insert(number) {
                    return Err(failure("Duplicate function addresses are not allowed"));
                }
                addresses.push(address);
            }
            return Ok(Self::Addresses { addresses });
        }
        let filter = args.get("filter").map_or(Ok(""), |value| {
            value.as_str().ok_or("The function filter must be text")
        })?;
        if filter.len() > 1024 {
            return Err(failure("The function filter exceeds 1024 bytes"));
        }
        Ok(Self::All {
            filter: filter.into(),
        })
    }

    pub(super) fn inputs(&self) -> (&str, Option<&[String]>) {
        match self {
            Self::All { filter } => (filter, None),
            Self::Addresses { addresses } => ("", Some(addresses)),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Context {
    pub project: String,
    pub provider: String,
    pub configuration: String,
}

impl Context {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(crate) fn new(project: &str, settings: &Settings) -> Result<Self> {
        let backend = settings.analysis_backend();
        backend.require_available()?;
        let configuration = configuration(settings);
        #[cfg(feature = "rea-rust-ghidra")]
        let configuration = if backend.retains_prepared_session() {
            let mut configuration = configuration;
            let identity = crate::analysis::rea_backend_identity(settings)?;
            configuration["rust_rea_backend"] = identity;
            configuration
        } else {
            configuration
        };
        let configuration = serde_json::to_string(&configuration)?;
        Ok(Self {
            project: project.into(),
            provider: settings.provider.clone(),
            configuration,
        })
    }

    pub(crate) fn matches(&self, job: &DecompilationJob) -> bool {
        self.project == job.project
            && self.provider == job.provider
            && self.configuration == job.options
    }

    pub(super) fn verify(&self, job: &DecompilationJob) -> Result<()> {
        if !self.matches(job) {
            return Err(failure(
                "This job belongs to a different project or analysis configuration. Restore its settings or start a new job.",
            ));
        }
        Ok(())
    }

    pub(super) fn job_id(&self, selection: &str, inventory: &str) -> Result<String> {
        let bytes = serde_json::to_vec(&(
            "prism-decompilation-v2",
            &self.project,
            &self.provider,
            &self.configuration,
            selection,
            inventory,
        ))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

fn configuration(settings: &Settings) -> Value {
    if settings.analysis_backend() == crate::settings::AnalysisBackend::RustGhidraRea {
        return json!({
            "format": 1,
            "provider": settings.provider,
            "memory_mib": settings.analysis_memory_mib,
            "timeout_seconds": settings.timeout_seconds
        });
    }
    json!({
        "format": 1,
        "provider": settings.provider,
        "rea": settings.rea,
        "rizin": settings.rizin,
        "jadx": settings.jadx,
        "memory_mib": settings.analysis_memory_mib,
        "timeout_seconds": settings.timeout_seconds
    })
}
