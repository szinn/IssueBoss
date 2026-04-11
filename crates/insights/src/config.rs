use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub repo: PathBuf,
    pub user: String,
    pub project: String,
}

impl Config {
    pub fn insights_dir() -> &'static Path {
        Path::new(".insights")
    }

    fn config_path() -> PathBuf {
        Self::insights_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        let content = std::fs::read_to_string(&path).with_context(|| format!("No config found at '{}'. Run 'insights init' first.", path.display()))?;
        toml::from_str(&content).context("Failed to parse .insights/config.toml")
    }

    pub fn write(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("Failed to create directory '{}'", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content).with_context(|| format!("Failed to write '{}'", path.display()))
    }

    pub fn project_lower(&self) -> String {
        self.project.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn with_insights_dir(f: impl FnOnce(&TempDir)) {
        let dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        f(&dir);
        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn load_missing_returns_error() {
        with_insights_dir(|_| {
            let err = Config::load().unwrap_err();
            assert!(err.to_string().contains("insights init"), "error should mention 'insights init'");
        });
    }

    #[test]
    fn write_creates_file() {
        with_insights_dir(|dir| {
            let cfg = Config {
                repo: PathBuf::from("/tmp/insights"),
                user: "alice".into(),
                project: "MyProject".into(),
            };
            cfg.write().unwrap();
            assert!(dir.path().join(".insights/config.toml").exists());
        });
    }

    #[test]
    fn round_trip() {
        with_insights_dir(|_| {
            let cfg = Config {
                repo: PathBuf::from("/tmp/insights"),
                user: "alice".into(),
                project: "MyProject".into(),
            };
            cfg.write().unwrap();
            let loaded = Config::load().unwrap();
            assert_eq!(loaded.user, "alice");
            assert_eq!(loaded.project, "MyProject");
            assert_eq!(loaded.repo, PathBuf::from("/tmp/insights"));
        });
    }

    #[test]
    fn project_lower() {
        let cfg = Config {
            repo: PathBuf::from("/tmp/insights"),
            user: "alice".into(),
            project: "IssueBoss".into(),
        };
        assert_eq!(cfg.project_lower(), "issueboss");
    }

    #[test]
    fn insights_dir_is_dotinsights() {
        assert_eq!(Config::insights_dir(), Path::new(".insights"));
    }
}
