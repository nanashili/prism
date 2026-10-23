use crate::decompilation::Context;
use sha2::{Digest, Sha256};

/// Returns the cache namespace for the complete analysis backend configuration.
///
/// A provider name alone is insufficient: executable paths, runtime identity,
/// memory limits, and timeouts can all change the result. Including the same
/// serialized configuration used by durable jobs keeps interactive and durable
/// reads on one provenance boundary.
pub(crate) fn namespace(context: &Context) -> String {
    let digest = Sha256::digest(context.configuration.as_bytes());
    format!("{}:cfg-{digest:x}", context.provider)
}
