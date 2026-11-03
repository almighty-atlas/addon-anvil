use crate::error::{AnvilError, Result};
use crate::model::{AppConfig, SCHEMA_VERSION};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Paths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| AnvilError::InvalidOperation("HOME nicht verfügbar".into()))?;
        fn xdg(var: &str, fallback: PathBuf) -> PathBuf {
            std::env::var_os(var)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or(fallback)
                .join("addon-anvil")
        }
        Ok(Self {
            config: xdg("XDG_CONFIG_HOME", home.join(".config")),
            data: xdg("XDG_DATA_HOME", home.join(".local/share")),
            cache: xdg("XDG_CACHE_HOME", home.join(".cache")),
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.json")
    }

    pub fn load(&self) -> Result<AppConfig> {
        let path = self.config_file();
        if !path.exists() {
            return Ok(AppConfig::default());
        }
        let content = std::fs::read(&path)?;
        let config: AppConfig = serde_json::from_slice(&content)?;
        if config.schema_version != SCHEMA_VERSION {
            return Err(AnvilError::InvalidOperation(format!(
                "Unbekannte Konfigurationsversion {} (unterstützt {})",
                config.schema_version, SCHEMA_VERSION
            )));
        }
        Ok(config)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        std::fs::create_dir_all(&self.config)?;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.config)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tmp.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        serde_json::to_writer_pretty(&mut tmp, config)?;
        tmp.write_all(b"\n")?;
        tmp.as_file_mut().sync_all()?;
        tmp.persist(self.config_file())
            .map_err(|err| AnvilError::Io(err.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_config_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: dir.path().to_owned(),
            data: dir.path().into(),
            cache: dir.path().into(),
        };
        std::fs::write(paths.config_file(), b"{broken").unwrap();
        assert!(paths.load().is_err());
        assert_eq!(std::fs::read(paths.config_file()).unwrap(), b"{broken");
    }

    #[test]
    fn round_trip_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: dir.path().to_owned(),
            data: dir.path().into(),
            cache: dir.path().into(),
        };
        paths.save(&AppConfig::default()).unwrap();
        assert_eq!(paths.load().unwrap().schema_version, 1);
    }
}
