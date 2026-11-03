use super::Catalog;
use crate::error::{AnvilError, Result};
use crate::model::{ProviderKind, Release, RemoteAddon};
use serde::Deserialize;

#[derive(Deserialize)]
struct Details {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    download: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    updated: Option<String>,
}

/// WoWInterface's MMOUI API is anonymous for project details, but does not
/// provide a reliably documented Forever-only search. Never infer support
/// for Forever from a generic WoWInterface listing.
pub fn parse_project(input: &str) -> Result<String> {
    let url = url::Url::parse(input.trim())
        .map_err(|_| AnvilError::Provider("Ungültige WoWInterface-URL".into()))?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some("www.wowinterface.com" | "wowinterface.com")
        )
    {
        return Err(AnvilError::Provider(
            "Nur WoWInterface-Projektseiten erlaubt".into(),
        ));
    }
    let name = url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or("");
    let id: String = name
        .strip_prefix("info")
        .unwrap_or("")
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if id.is_empty() {
        return Err(AnvilError::Provider("Keine Projekt-ID in URL".into()));
    }
    Ok(id)
}

pub async fn latest(cat: &Catalog, project: &str) -> Result<RemoteAddon> {
    let id = project
        .parse::<u64>()
        .map_err(|_| AnvilError::Provider("Ungültige WoWInterface-ID".into()))?;
    let url = format!("{}/v3/game/WOW/filedetails/{id}.json", cat.wowi_base);
    let data: Details = cat.get(&url, None).await?;
    if data.id.is_some_and(|remote| remote != id) {
        return Err(AnvilError::Provider(
            "API lieferte falsche Projekt-ID".into(),
        ));
    }
    let downloadable = data
        .download
        .filter(|s| url::Url::parse(s).is_ok_and(|u| u.scheme() == "https"));
    Ok(RemoteAddon {
        provider: ProviderKind::WoWInterface,
        project_id: project.into(),
        name: data.title.unwrap_or_else(|| format!("WoWI {id}")),
        summary: String::new(),
        project_url: data
            .url
            .filter(|s| s.starts_with("https://www.wowinterface.com/"))
            .unwrap_or_else(|| format!("https://www.wowinterface.com/downloads/info{id}.html")),
        release: Some(Release {
            id: data.version.clone().unwrap_or_default(),
            version: data.version.unwrap_or_else(|| "unbekannt".into()),
            published_at: data.updated,
            download_url: downloadable.clone(),
        }),
        restriction: if downloadable.is_some() {
            None
        } else {
            Some(
                "Kein öffentlicher HTTPS-Download in den API-Metadaten. Öffne die Projektseite."
                    .into(),
            )
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wowi_id_from_official_page() {
        assert_eq!(
            parse_project("https://www.wowinterface.com/downloads/info25043-APIInterface.html")
                .unwrap(),
            "25043"
        );
        assert!(parse_project("https://evil.example/downloads/info12.html").is_err());
    }
}
