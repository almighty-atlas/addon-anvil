use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    Local,
    GitHub,
    WoWInterface,
    CurseForge,
}

impl ProviderKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "Lokale Datei",
            Self::GitHub => "GitHub",
            Self::WoWInterface => "WoWInterface",
            Self::CurseForge => "CurseForge",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameInstallation {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub interface: Option<u32>,
}

impl GameInstallation {
    pub fn addons_dir(&self) -> PathBuf {
        self.path.join("Interface").join("AddOns")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedAddon {
    pub installation_id: String,
    pub provider: ProviderKind,
    pub project_id: String,
    pub name: String,
    pub release_id: String,
    pub folders: Vec<String>,
    pub project_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    pub installations: Vec<GameInstallation>,
    pub selected_installation: Option<String>,
    pub managed: Vec<ManagedAddon>,
    pub last_check: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            installations: Vec::new(),
            selected_installation: None,
            managed: Vec::new(),
            last_check: None,
        }
    }
}

impl AppConfig {
    pub fn selected(&self) -> Option<&GameInstallation> {
        let id = self.selected_installation.as_ref()?;
        self.installations.iter().find(|item| &item.id == id)
    }

    pub fn owned_folder(&self, install_id: &str, folder: &str) -> Option<&ManagedAddon> {
        self.managed
            .iter()
            .find(|m| m.installation_id == install_id && m.folders.iter().any(|f| f == folder))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TocMetadata {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub version: Option<String>,
    pub interfaces: Vec<u32>,
    pub dependencies: Vec<String>,
    pub optional_dependencies: Vec<String>,
    pub wowi_id: Option<String>,
    pub website: Option<String>,
    pub forever_suffix: bool,
}

#[derive(Debug, Clone)]
pub struct ScannedAddon {
    pub folder: String,
    pub toc: TocMetadata,
    pub compatible: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Release {
    pub id: String,
    pub version: String,
    pub download_url: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteAddon {
    pub provider: ProviderKind,
    pub project_id: String,
    pub name: String,
    pub summary: String,
    pub project_url: String,
    pub release: Option<Release>,
    pub restriction: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub provider: ProviderKind,
    pub project_id: String,
    pub name: String,
    pub release_id: String,
    pub project_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_round_trip() {
        let cfg = AppConfig::default();
        let encoded = serde_json::to_string(&cfg).unwrap();
        let decoded: AppConfig = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.schema_version, 1);
    }
}
