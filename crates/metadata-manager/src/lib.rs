use afs_common::TaskMetadata;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct MetadataRegistry {
    root: PathBuf,
}

impl MetadataRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create {}", self.root.display()))?;
        Ok(())
    }

    pub fn task_path(&self, pid: i32) -> PathBuf {
        self.root.join(format!("{pid}.json"))
    }

    pub fn ready_path(&self, pid: i32) -> PathBuf {
        self.root.join(format!("{pid}.ready"))
    }

    pub fn result_path(&self, pid: i32) -> PathBuf {
        self.root.join(format!("{pid}.result.json"))
    }

    pub fn register(&self, metadata: &TaskMetadata) -> Result<PathBuf> {
        metadata.validate().map_err(anyhow::Error::msg)?;
        self.initialize()?;
        let path = self.task_path(metadata.process_id);
        let tmp = self.root.join(format!(".{}.json.tmp", metadata.process_id));
        let data = serde_json::to_vec_pretty(metadata)?;
        {
            let mut file = fs::File::create(&tmp)
                .with_context(|| format!("failed to create {}", tmp.display()))?;
            file.write_all(&data)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path).with_context(|| format!("failed to install {}", path.display()))?;
        fs::write(self.ready_path(metadata.process_id), b"ready\n")?;
        Ok(path)
    }

    pub fn load(&self, pid: i32) -> Result<Option<TaskMetadata>> {
        let path = self.task_path(pid);
        match fs::read(&path) {
            Ok(data) => {
                let metadata = serde_json::from_slice::<TaskMetadata>(&data)
                    .with_context(|| format!("invalid metadata {}", path.display()))?;
                Ok(Some(metadata))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn wait_ready(&self, pid: i32, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let path = self.ready_path(pid);
        while !path.exists() {
            if start.elapsed() >= timeout {
                anyhow::bail!("timed out waiting for {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    pub fn unregister(&self, pid: i32) -> Result<()> {
        for path in [self.task_path(pid), self.ready_path(pid)] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err)
                        .with_context(|| format!("failed to remove {}", path.display()));
                }
            }
        }
        Ok(())
    }

    pub fn load_all(&self) -> Result<HashMap<i32, TaskMetadata>> {
        self.initialize()?;
        let mut out = HashMap::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            let pid: i32 = match stem.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(metadata) = self.load(pid)? {
                out.insert(pid, metadata);
            }
        }
        Ok(out)
    }

    /// Remove all registry artifacts before a new isolated experiment run.
    ///
    /// This must only be used when no other experiment shares the registry
    /// directory. The default scripts allocate one registry per machine and
    /// serialize runs.
    pub fn clear(&self) -> Result<usize> {
        self.initialize()?;
        let mut removed = 0usize;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err)
                        .with_context(|| format!("failed to remove {}", path.display()));
                }
            }
        }
        Ok(removed)
    }

    pub fn cleanup_stale(&self, older_than: Duration) -> Result<usize> {
        self.initialize()?;
        let now = SystemTime::now();
        let mut removed = 0usize;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            let modified = match metadata.modified() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if now.duration_since(modified).unwrap_or_default() < older_than {
                continue;
            }
            if fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use afs_common::{TaskMetadata, WorkloadClass, WorkloadKind};

    #[test]
    fn registry_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let registry = MetadataRegistry::new(dir.path());
        let metadata = TaskMetadata {
            task_id: 1,
            process_id: 123,
            workflow_id: 2,
            stage_id: 3,
            workload_class: WorkloadClass::Critical,
            kind: WorkloadKind::Cpu,
            release_time_ns: 100,
            absolute_deadline_ns: Some(200),
            estimated_total_runtime_ns: 50,
            estimated_remaining_runtime_ns: 50,
            application_priority: 1.0,
            name: "test".into(),
        };
        registry.register(&metadata).unwrap();
        let loaded = registry.load(123).unwrap().unwrap();
        assert_eq!(loaded.task_id, 1);
        registry.unregister(123).unwrap();
        assert!(registry.load(123).unwrap().is_none());
    }
}
