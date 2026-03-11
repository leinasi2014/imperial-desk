//! Local filesystem state for browser profiles and recent session metadata.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use web_llm_core::{Result, WebLlmError};

const APP_DIR_NAME: &str = "web-llm-adapter-rust";

#[derive(Debug, Clone)]
pub struct StatePaths {
    pub root_dir: PathBuf,
    pub provider_dir: PathBuf,
    pub profile_dir: PathBuf,
    pub provider_config_path: PathBuf,
    pub recent_session_path: PathBuf,
}

impl StatePaths {
    pub fn resolve(provider_id: &str, override_profile_dir: Option<&Path>) -> Result<Self> {
        let root_dir = dirs::data_local_dir()
            .ok_or(WebLlmError::DataDirUnavailable)?
            .join(APP_DIR_NAME);
        let provider_dir = root_dir.join(provider_id);
        let profile_dir = override_profile_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| provider_dir.join("profile"));
        let provider_config_path = provider_dir.join("provider-config.json");
        let recent_session_path = provider_dir.join("recent-session.json");

        Ok(Self {
            root_dir,
            provider_dir,
            profile_dir,
            provider_config_path,
            recent_session_path,
        })
    }

    pub fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir)?;
        fs::create_dir_all(&self.provider_dir)?;
        fs::create_dir_all(&self.profile_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSessionRecord {
    pub session_id: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub phone_number: Option<String>,
}

pub fn load_recent_session(path: &Path) -> Result<Option<RecentSessionRecord>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let record = serde_json::from_str(&content)?;
    Ok(Some(record))
}

pub fn save_recent_session(path: &Path, record: &RecentSessionRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(record)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn clear_recent_session(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

pub fn load_provider_config(path: &Path) -> Result<Option<ProviderConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let record = serde_json::from_str(&content)?;
    Ok(Some(record))
}

pub fn save_provider_config(path: &Path, record: &ProviderConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(record)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn clear_provider_config(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}
