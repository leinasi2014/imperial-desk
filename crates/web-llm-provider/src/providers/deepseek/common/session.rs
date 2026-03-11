use std::path::Path;

use web_llm_core::{Result, WebLlmError};
use web_llm_state::{
    clear_recent_session, load_recent_session, save_recent_session, RecentSessionRecord,
};

pub fn sanitize_session_id(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub fn extract_session_id_from_url(url: &str) -> Option<String> {
    let marker = "/a/chat/s/";
    let index = url.find(marker)?;
    let remainder = &url[index + marker.len()..];
    let session_id = remainder
        .split(['?', '#', '/'])
        .next()
        .unwrap_or_default()
        .trim();
    if session_id.is_empty() {
        None
    } else {
        Some(session_id.to_owned())
    }
}

#[must_use]
pub fn build_session_url(base_url: &str, session_id: &str) -> String {
    format!("{}/a/chat/s/{}", base_url.trim_end_matches('/'), session_id)
}

pub fn remember_recent_session(
    path: &Path,
    session_id: Option<&str>,
    url: Option<String>,
) -> Result<()> {
    match session_id {
        Some(session_id) => save_recent_session(
            path,
            &RecentSessionRecord {
                session_id: Some(session_id.to_owned()),
                updated_at: None,
                url,
            },
        ),
        None => clear_recent_session(path),
    }
}

pub fn clear_recent_session_if_matches(path: &Path, session_id: &str) -> Result<()> {
    if let Some(record) = load_recent_session(path)? {
        if record.session_id.as_deref() == Some(session_id) {
            clear_recent_session(path)?;
        }
    }

    Ok(())
}

pub fn load_required_recent_session_id(path: &Path) -> Result<String> {
    let record = load_recent_session(path)?
        .ok_or_else(|| WebLlmError::browser("no recent DeepSeek chat session is recorded"))?;
    record
        .session_id
        .ok_or_else(|| WebLlmError::browser("no recent DeepSeek chat session is recorded"))
}
