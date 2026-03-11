use serde::Deserialize;

pub const DEFAULT_BASE_URL: &str = "https://chat.deepseek.com/";
pub const KNOWN_MODELS: &[&str] = &["DeepSeek-R1", "DeepSeek-V3", "R1", "V3"];

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorizedMutationResult {
    pub error: Option<String>,
    pub ok: bool,
    pub status: u64,
    #[serde(rename = "data")]
    pub _data: Option<serde_json::Value>,
}
