pub mod curseforge;
pub mod github;
pub mod wowinterface;

use crate::archive::MAX_ARCHIVE_SIZE;
use crate::error::{AnvilError, Result};
use crate::model::{ProviderKind, Release, RemoteAddon};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone)]
pub struct Catalog {
    client: Client,
    github_base: String,
    curseforge_base: String,
    wowi_base: String,
}

impl Catalog {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("AddonAnvil/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(35))
            .build()?;
        Ok(Self {
            client,
            github_base: "https://api.github.com".into(),
            curseforge_base: "https://api.curseforge.com".into(),
            wowi_base: "https://api.mmoui.com".into(),
        })
    }

    pub async fn search(&self, provider: ProviderKind, text: &str) -> Result<Vec<RemoteAddon>> {
        match provider {
            ProviderKind::GitHub => github::search(self, text).await,
            ProviderKind::WoWInterface => Err(AnvilError::Provider(
                "Die öffentliche WoWInterface-API bietet keine verifizierte Forever-Suche. Bitte eine Projekt-URL hinzufügen.".into()
            )),
            ProviderKind::CurseForge => curseforge::search(self, text).await,
            ProviderKind::Local => Ok(Vec::new()),
        }
    }

    pub async fn latest(&self, provider: ProviderKind, project_id: &str) -> Result<RemoteAddon> {
        match provider {
            ProviderKind::GitHub => github::latest(self, project_id).await,
            ProviderKind::WoWInterface => wowinterface::latest(self, project_id).await,
            ProviderKind::CurseForge => curseforge::latest(self, project_id).await,
            ProviderKind::Local => Err(AnvilError::Provider(
                "Lokale ZIPs haben keine Update-Quelle".into(),
            )),
        }
    }

    async fn get<T: DeserializeOwned>(&self, url: &str, key: Option<&str>) -> Result<T> {
        let mut request = self.client.get(url);
        if let Some(key) = key {
            request = request.header("x-api-key", key);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let reason = match status {
                StatusCode::TOO_MANY_REQUESTS => "Rate-Limit erreicht",
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    "API-Zugriff verweigert oder Rate-Limit erreicht"
                }
                StatusCode::NOT_FOUND => "Projekt oder Datei nicht gefunden",
                _ => "API-Abfrage fehlgeschlagen",
            };
            return Err(AnvilError::Provider(format!("{reason} (HTTP {status})")));
        }
        let bytes = response.bytes().await?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(AnvilError::TooLarge);
        }
        serde_json::from_slice(&bytes).map_err(AnvilError::Config)
    }

    pub async fn download(&self, url: &str, cache_dir: &Path) -> Result<PathBuf> {
        let parsed = url::Url::parse(url)
            .map_err(|_| AnvilError::Provider("Ungültige Download-URL".into()))?;
        if parsed.scheme() != "https" || parsed.host_str().is_none() {
            return Err(AnvilError::Provider("Nur HTTPS-Downloads erlaubt".into()));
        }
        let response = self.client.get(parsed).send().await?;
        if !response.status().is_success() {
            return Err(AnvilError::Provider(format!(
                "Download nicht verfügbar: HTTP {}",
                response.status()
            )));
        }
        if response.url().scheme() != "https" {
            return Err(AnvilError::Provider("Unsichere HTTP-Weiterleitung".into()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_ARCHIVE_SIZE)
        {
            return Err(AnvilError::TooLarge);
        }
        std::fs::create_dir_all(cache_dir)?;
        let file = tempfile::Builder::new()
            .prefix("addon-anvil-")
            .suffix(".zip")
            .tempfile_in(cache_dir)?;
        let path = file.into_temp_path();
        let mut output = tokio::fs::File::create(&path).await?;
        use tokio::io::AsyncWriteExt;
        let mut received = 0_u64;
        let mut stream = response.bytes_stream();
        while let Some(next) = stream.next().await {
            let chunk = next?;
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or(AnvilError::TooLarge)?;
            if received > MAX_ARCHIVE_SIZE {
                return Err(AnvilError::TooLarge);
            }
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        drop(output);
        path.keep().map_err(|e| AnvilError::Io(e.error))
    }
}

pub fn release_is_newer(installed_id: &str, latest: &Release) -> bool {
    !installed_id.is_empty() && installed_id != latest.id
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_ids_for_updates() {
        let release = Release {
            id: "987".into(),
            version: "v1.5".into(),
            download_url: Some("https://example.test/archive.zip".into()),
            published_at: None,
        };
        assert!(!release_is_newer("987", &release));
        assert!(release_is_newer("986", &release));
    }
    #[tokio::test]
    async fn handles_mock_http_failures() {
        let mock = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(429))
            .mount(&mock)
            .await;
        let cat = Catalog::new().unwrap();
        let response: Result<serde_json::Value> = cat.get(&mock.uri(), None).await;
        assert!(matches!(response, Err(AnvilError::Provider(_))));
    }
}
