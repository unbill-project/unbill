use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

#[derive(Debug, Clone, Copy)]
pub struct UnbillPath {
    env_var: &'static str,
    qualifier: &'static str,
    organization: &'static str,
    application: &'static str,
}

pub static UNBILL_PATH: UnbillPath = UnbillPath::new();

impl UnbillPath {
    pub const fn new() -> Self {
        Self {
            env_var: "UNBILL_DATA_DIR",
            qualifier: "",
            organization: "",
            application: "unbill",
        }
    }

    pub fn data_dir(&self) -> Result<PathBuf> {
        if let Some(path) = std::env::var_os(self.env_var) {
            return Ok(PathBuf::from(path));
        }

        ProjectDirs::from(self.qualifier, self.organization, self.application)
            .map(|dirs| dirs.data_dir().to_path_buf())
            .context("could not resolve data directory")
    }

    pub fn ensure_data_dir(&self) -> Result<PathBuf> {
        let path = self.data_dir()?;
        std::fs::create_dir_all(&path)
            .with_context(|| format!("unable to create data directory at {}", path.display()))?;
        Ok(path)
    }
}
