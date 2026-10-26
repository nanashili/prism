//! Process-wide profiling lifecycle for Prism entry points.

#[cfg(feature = "profiling")]
use hotpath::HotpathGuardBuilder;

/// Owns the process-wide profiling session until the entry point has completed.
pub struct Guard {
    #[cfg(feature = "profiling")]
    _session: hotpath::HotpathGuard,
}

impl std::fmt::Debug for Guard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Guard").finish_non_exhaustive()
    }
}

/// Starts the profiler for a normal Prism process.
#[must_use]
pub fn start() -> Guard {
    Guard {
        #[cfg(feature = "profiling")]
        _session: HotpathGuardBuilder::new("prism").functions_limit(0).build(),
    }
}

/// Starts the profiler for the MCP process without writing a report to stdout.
#[must_use]
pub fn start_mcp() -> Guard {
    #[cfg(not(feature = "profiling"))]
    return Guard {};

    #[cfg(feature = "profiling")]
    {
        // Live MCP snapshots use the builder limit, independently of the
        // environment limit used when hotpath writes its final report.
        let builder = HotpathGuardBuilder::new("prism-mcp").functions_limit(0);

        #[cfg(unix)]
        let builder = builder.output_path("/dev/stderr");

        #[cfg(windows)]
        let builder = builder.output_path(std::env::temp_dir().join(format!(
            "prism-mcp-hotpath-report-{}.txt",
            std::process::id()
        )));

        Guard {
            _session: builder.build(),
        }
    }
}
