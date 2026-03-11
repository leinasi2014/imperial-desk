//! Provider registry and implementations for web and API-backed model adapters.

mod registry;

pub mod providers;

pub use registry::{provider_definition, provider_definitions, DEFAULT_PROVIDER_ID};
