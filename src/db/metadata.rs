use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub mtime: u64,
    pub size: u64,
    pub hash: u64,
    pub indexed_at: u64,
}

impl FileMetadata {
    pub fn from_file(path: &Path, content: &str) -> Self {
        let metadata = fs::metadata(path).ok();
        let mtime = metadata.as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let size = metadata.map(|m| m.len()).unwrap_or(0);
        let hash = Self::compute_hash(content);
        let indexed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            path: path.to_string_lossy().to_string(),
            mtime,
            size,
            hash,
            indexed_at,
        }
    }

    fn compute_hash(content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    pub fn needs_reindex(&self, path: &Path) -> bool {
        let current_metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return true,
        };

        let current_mtime = current_metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let current_size = current_metadata.len();

        current_mtime > self.mtime || current_size != self.size
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataStore {
    pub files: HashMap<String, FileMetadata>,
    pub version: u32,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            version: 1,
        }
    }

    pub fn update(&mut self, metadata: FileMetadata) {
        self.files.insert(metadata.path.clone(), metadata);
    }

    pub fn remove(&mut self, path: &str) {
        self.files.remove(path);
    }

    pub fn get(&self, path: &str) -> Option<&FileMetadata> {
        self.files.get(path)
    }

    pub fn needs_reindex(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        match self.files.get(&path_str) {
            Some(meta) => meta.needs_reindex(path),
            None => true,
        }
    }

    pub fn cleanup_removed(&mut self, existing_paths: &[String]) {
        let existing: std::collections::HashSet<_> = existing_paths.iter().cloned().collect();
        self.files.retain(|k, _| existing.contains(k));
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !Path::new(path).exists() {
            return Ok(Self::new());
        }
        let data = fs::read_to_string(path)?;
        let store: Self = serde_json::from_str(&data)?;
        Ok(store)
    }
}