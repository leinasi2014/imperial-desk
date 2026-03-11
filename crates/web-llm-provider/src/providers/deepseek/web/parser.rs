use crate::providers::deepseek::common::models::KNOWN_MODELS;

pub fn sanitize_response(value: &str) -> String {
    let mut normalized = value.replace("\r\n", "\n");
    while normalized.contains("\n\n\n") {
        normalized = normalized.replace("\n\n\n", "\n\n");
    }
    normalized.trim().to_owned()
}

pub fn detect_model_in_text(text: &str) -> Option<String> {
    KNOWN_MODELS
        .iter()
        .find(|candidate| text.contains(**candidate))
        .map(|candidate| (*candidate).to_owned())
}
