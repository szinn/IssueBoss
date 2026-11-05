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
        let mut config: Self = toml::from_str(&content).context("Failed to parse .insights/config.toml")?;
        config.repo = normalize_path(&config.repo);
        Ok(config)
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

/// Lexically normalize a path: collapse repeated separators (`//Users` →
/// `/Users`) and drop `.` components, without touching the filesystem.
///
/// Symlink targets are built by joining onto the repo path, so an
/// unnormalized repo path produces unnormalized link targets (e.g.
/// `//Users/...`) that some tools refuse to follow.
pub fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn normalize_path_collapses_leading_double_slash() {
        assert_eq!(normalize_path(Path::new("//Users/alice/Insights")), PathBuf::from("/Users/alice/Insights"));
    }

    #[test]
    fn normalize_path_collapses_inner_separators_and_dots() {
        assert_eq!(normalize_path(Path::new("/Users//alice/./Insights/")), PathBuf::from("/Users/alice/Insights"));
    }

    #[test]
    fn load_normalizes_repo_path() {
        with_insights_dir(|dir| {
            std::fs::create_dir_all(dir.path().join(".insights")).unwrap();
            std::fs::write(
                dir.path().join(".insights/config.toml"),
                "repo = \"//tmp/insights\"\nuser = \"alice\"\nproject = \"MyProject\"\n",
            )
            .unwrap();
            let loaded = Config::load().unwrap();
            assert_eq!(loaded.repo, PathBuf::from("/tmp/insights"), "leading // should be collapsed on load");
        });
    }

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
