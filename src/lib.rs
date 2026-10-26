//! Shared MCP protocol definitions and helper transport for the Prism desktop application.

// Define the allocator once in the shared crate so both entry points collect
// allocations without installing competing allocators in the desktop binary.
#[cfg(feature = "profiling-alloc")]
#[global_allocator]
static PROFILING_ALLOCATOR: hotpath::CountingAllocator = hotpath::CountingAllocator::new();

mod error;
mod mcp_error;

pub mod mcp;
pub mod profiling;

pub use mcp_error::Error;

/// JSON values exchanged by the MCP protocol and tool catalog.
pub use serde_json::Value;
