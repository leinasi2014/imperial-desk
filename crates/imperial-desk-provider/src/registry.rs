use imperial_desk_core::ProviderDefinition;

use crate::providers::deepseek;

pub const DEFAULT_PROVIDER_ID: &str = deepseek::web::PROVIDER_ID;

#[must_use]
pub fn provider_definitions() -> [ProviderDefinition; 1] {
    [deepseek::web::definition()]
}

#[must_use]
pub fn provider_definition(provider_id: &str) -> Option<ProviderDefinition> {
    provider_definitions()
        .into_iter()
        .find(|definition| definition.metadata.id == provider_id)
}
