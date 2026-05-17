use std::path::PathBuf;

use directories::ProjectDirs;
use unbill_model::UnbillError;

#[derive(Debug, Clone, Copy)]
pub struct UnbillPath {
    env_var: &'static str,
    qualifier: &'static str,
    organization: &'static str,
    application: &'static str,
}

pub static UNBILL_PATH: UnbillPath = UnbillPath::new();

impl Default for UnbillPath {
    fn default() -> Self {
        Self::new()
    }
}

impl UnbillPath {
    pub const fn new() -> Self {
        Self {
            env_var: "UNBILL_DATA_DIR",
            qualifier: "",
            organization: "",
            application: "unbill",
        }
    }

    pub fn data_dir(&self) -> Result<PathBuf, UnbillError> {
        if let Some(path) = std::env::var_os(self.env_var) {
            return Ok(PathBuf::from(path));
        }

        ProjectDirs::from(self.qualifier, self.organization, self.application)
            .map(|dirs| dirs.data_dir().to_path_buf())
            .ok_or_else(|| UnbillError::Config("could not resolve data directory".into()))
    }

    /// Path of the Unix/named-pipe socket the daemon listens on.
    /// Respects `UNBILL_SOCKET` env override.
    pub fn socket_path(&self) -> Result<PathBuf, UnbillError> {
        if let Some(path) = std::env::var_os("UNBILL_SOCKET") {
            return Ok(PathBuf::from(path));
        }
        Ok(self.data_dir()?.join("unbill.sock"))
    }

    pub fn ensure_data_dir(&self) -> Result<PathBuf, UnbillError> {
        let path = self.data_dir()?;
        std::fs::create_dir_all(&path).map_err(|e| {
            UnbillError::Config(format!(
                "unable to create data directory at {}: {e}",
                path.display()
            ))
        })?;
        Ok(path)
    }
}
