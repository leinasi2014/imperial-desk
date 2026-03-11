//! DeepSeek web provider implementation.

mod mutations;
mod parser;
pub mod provider;
mod selectors;

pub use provider::{definition, PROVIDER_ID};
