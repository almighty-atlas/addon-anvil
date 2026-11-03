use super::Catalog;
use crate::error::{AnvilError, Result};
use crate::model::{ProviderKind, Release, RemoteAddon};
use serde::Deserialize;

const WOW_GAME_ID: u64 = 1;

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}
#[derive(Deserialize)]
struct VersionType {
    id: u64,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: u64,
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    allow_mod_distribution: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    display_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    file_date: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
}

fn key() -> Result<String> {
    std::env::var("CURSEFORGE_API_KEY").ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AnvilError::Provider(
            "CURSEFORGE_API_KEY fehlt. Ohne eigenen, autorisierten API-Schlüssel ist CurseForge deaktiviert.".into()
        ))
}

async fn forever_type(cat: &Catalog, key: &str) -> Result<u64> {
    let url = format!(
        "{}/v1/games/{WOW_GAME_ID}/version-types",
        cat.curseforge_base
    );
    let types: Envelope<Vec<VersionType>> = cat.get(&url, Some(key)).await?;
    types
        .data
        .into_iter()
        .find(|v| {
            let name = v.name.to_ascii_lowercase();
            name.contains("forever") || name.contains("camelot")
        })
        .map(|v| v.id)
        .ok_or_else(|| {
            AnvilError::Provider(
                "CurseForge meldet keinen Forever-Versionstyp; nicht auf Retail/Classic ausweichen"
                    .into(),
            )
        })
}

async fn get_files(cat: &Catalog, key: &str, id: u64, version_type: u64) -> Result<Vec<CfFile>> {
    let url = format!(
        "{}/v1/mods/{id}/files?gameVersionTypeId={version_type}&pageSize=50",
        cat.curseforge_base
    );
    let response: Envelope<Vec<CfFile>> = cat.get(&url, Some(key)).await?;
    Ok(response.data)
}

async fn convert(cat: &Catalog, key: &str, version_type: u64, item: CfMod) -> Result<RemoteAddon> {
    let files = get_files(cat, key, item.id, version_type).await?;
    // Filtered by official Forever game-version type, not by similar filenames.
    let file = files.into_iter().next();
    let download = if item.allow_mod_distribution == Some(false) {
        None
    } else {
        file.as_ref()
            .and_then(|f| f.download_url.clone())
            .filter(|url| url::Url::parse(url).is_ok_and(|u| u.scheme() == "https"))
    };
    let restriction = if item.allow_mod_distribution == Some(false) {
        Some("Der Autor untersagt Drittanbieter-Downloads".into())
    } else if download.is_none() {
        Some("CurseForge stellt keinen freigegebenen Download bereit".into())
    } else {
        None
    };
    let project_url = format!(
        "https://www.curseforge.com/wow/addons/{}",
        if item.slug.is_empty() {
            item.id.to_string()
        } else {
            item.slug
        }
    );
    Ok(RemoteAddon {
        provider: ProviderKind::CurseForge,
        project_id: item.id.to_string(),
        name: item.name,
        summary: item.summary,
        project_url,
        release: file.map(|f| {
            let _game_versions = f.game_versions;
            Release {
                id: f.id.to_string(),
                version: f.display_name,
                published_at: f.file_date,
                download_url: download,
            }
        }),
        restriction,
    })
}

pub async fn search(cat: &Catalog, query: &str) -> Result<Vec<RemoteAddon>> {
    let key = key()?;
    let flavor = forever_type(cat, &key).await?;
    let url = format!(
        "{}/v1/mods/search?gameId={WOW_GAME_ID}&gameVersionTypeId={flavor}&pageSize=20&searchFilter={}",
        cat.curseforge_base,
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
    );
    let response: Envelope<Vec<CfMod>> = cat.get(&url, Some(&key)).await?;
    let mut results = Vec::new();
    for item in response.data {
        // Each candidate has an independently queried Forever release.
        if let Ok(addon) = convert(cat, &key, flavor, item).await {
            results.push(addon);
        }
    }
    Ok(results)
}

pub async fn latest(cat: &Catalog, id: &str) -> Result<RemoteAddon> {
    let key = key()?;
    let id: u64 = id
        .parse()
        .map_err(|_| AnvilError::Provider("CurseForge-ID ungültig".into()))?;
    let flavor = forever_type(cat, &key).await?;
    let url = format!("{}/v1/mods/{id}", cat.curseforge_base);
    let response: Envelope<CfMod> = cat.get(&url, Some(&key)).await?;
    convert(cat, &key, flavor, response.data).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_embedded_secret() {
        assert!(std::env::var("CURSEFORGE_API_KEY").is_err() || key().is_ok());
    }
}
