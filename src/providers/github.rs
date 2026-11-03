use super::Catalog;
use crate::error::{AnvilError, Result};
use crate::model::{ProviderKind, Release, RemoteAddon};
use serde::Deserialize;

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    name: String,
    description: Option<String>,
    html_url: String,
}
#[derive(Deserialize)]
struct Repos {
    items: Vec<Repo>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}
#[derive(Deserialize)]
struct GithubRelease {
    id: u64,
    tag_name: String,
    published_at: Option<String>,
    draft: bool,
    assets: Vec<Asset>,
}

pub fn parse_project(input: &str) -> Result<String> {
    let input = input.trim();
    let url = url::Url::parse(input).map_err(|_| {
        AnvilError::Provider("GitHub-URL muss mit https://github.com/ beginnen".into())
    })?;
    if url.scheme() != "https" || url.host_str() != Some("github.com") {
        return Err(AnvilError::Provider(
            "Nur offizielle GitHub-Projekt-URLs erlaubt".into(),
        ));
    }
    let parts: Vec<_> = url
        .path_segments()
        .ok_or_else(|| AnvilError::Provider("Ungültige URL".into()))?
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err(AnvilError::Provider("owner/repo fehlt".into()));
    }
    let owner = parts[0];
    let repo = parts[1].trim_end_matches(".git");
    if [owner, repo].iter().any(|s| {
        s.is_empty()
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    }) {
        return Err(AnvilError::Provider(
            "GitHub-Eigentümer oder Repository ungültig".into(),
        ));
    }
    Ok(format!("{owner}/{repo}"))
}

pub async fn search(cat: &Catalog, text: &str) -> Result<Vec<RemoteAddon>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let query = format!("wow addon {}", text.trim());
    let url = format!(
        "{}/search/repositories?q={}&per_page=20",
        cat.github_base,
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
    );
    let results: Repos = cat.get(&url, None).await?;
    Ok(results
        .items
        .into_iter()
        .map(|repo| RemoteAddon {
            provider: ProviderKind::GitHub,
            project_id: repo.full_name,
            name: repo.name,
            summary: repo.description.unwrap_or_default(),
            project_url: repo.html_url,
            release: None,
            restriction: None,
        })
        .collect())
}

pub async fn latest(cat: &Catalog, project: &str) -> Result<RemoteAddon> {
    let segments: Vec<_> = project.split('/').collect();
    if segments.len() != 2
        || segments.iter().any(|s| {
            s.is_empty()
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
    {
        return Err(AnvilError::Provider(
            "Ungültige GitHub-Projektkennung".into(),
        ));
    }
    let api = format!("{}/repos/{project}/releases", cat.github_base);
    let releases: Vec<GithubRelease> = cat.get(&api, None).await?;
    let release = releases
        .into_iter()
        .find(|r| !r.draft)
        .ok_or_else(|| AnvilError::Provider("Keine veröffentlichten GitHub-Releases".into()))?;
    let zips: Vec<_> = release
        .assets
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().ends_with(".zip"))
        .collect();
    let specific: Vec<_> = zips
        .iter()
        .filter(|a| {
            let name = a.name.to_ascii_lowercase();
            (name.contains("forever") || name.contains("camelot"))
                && !name.contains("retail")
                && !name.contains("classic-era")
        })
        .collect();
    // A single explicitly Forever-tagged ZIP is safe to select; a sole generic
    // ZIP still requires a compatible TOC during installation.
    let asset = match (specific.as_slice(), zips.as_slice()) {
        ([asset], _) => Some(*asset),
        ([], [asset]) => Some(*asset),
        _ => None,
    };
    let reason = if asset.is_none() {
        Some(
            "Kein eindeutig auswählbares ZIP-Release (mehrere Varianten oder keine ZIP-Datei)"
                .into(),
        )
    } else {
        None
    };
    Ok(RemoteAddon {
        provider: ProviderKind::GitHub,
        project_id: project.into(),
        name: segments[1].into(),
        summary: String::new(),
        project_url: format!("https://github.com/{project}"),
        release: Some(Release {
            id: release.id.to_string(),
            version: release.tag_name,
            published_at: release.published_at,
            download_url: asset.map(|a| a.browser_download_url.clone()),
        }),
        restriction: reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_url() {
        assert_eq!(
            parse_project("https://github.com/BigWigsMods/BigWigs/releases").unwrap(),
            "BigWigsMods/BigWigs"
        );
        assert!(parse_project("http://github.com/owner/repo").is_err());
        assert!(parse_project("https://example.com/owner/repo").is_err());
    }
}
