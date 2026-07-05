use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaUploadState {
    pub video_url: String,
    pub caption: String,
    pub platform: String,
    pub container_id: Option<String>,
    pub media_id: Option<String>,
    pub step: String,
}

fn state_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hermes/data/uploads")
}

fn state_path(video_url: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    video_url.hash(&mut hasher);
    state_dir().join(format!("meta_upload_{:016x}.json", hasher.finish()))
}

#[allow(dead_code)]
pub async fn load(video_url: &str) -> Option<MetaUploadState> {
    let path = state_path(video_url);
    if !path.exists() {
        return None;
    }
    let bytes = tokio::fs::read(&path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub async fn save(state: &MetaUploadState) -> anyhow::Result<()> {
    let path = state_path(&state.video_url);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, serde_json::to_vec_pretty(state)?).await?;
    Ok(())
}

pub async fn clear(video_url: &str) {
    let _ = tokio::fs::remove_file(state_path(video_url)).await;
}
