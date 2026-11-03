use crate::error::{AnvilError, Result};
use crate::model::GameInstallation;
use crate::toc;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn validate(candidate: &Path) -> Result<GameInstallation> {
    let root = candidate.canonicalize()?;
    if !root.is_dir() { return Err(AnvilError::InvalidGame(root.display().to_string())); }
    // A selected client folder is required; do not silently pick between client flavours.
    let leaf = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let addons = root.join("Interface").join("AddOns");
    let evidence = root.join("Data").is_dir()
        || root.join("Wow.exe").is_file()
        || root.join("WowClassic.exe").is_file()
        || root.join("World of Warcraft.exe").is_file()
        || addons.is_dir();
    if !evidence {
        return Err(AnvilError::InvalidGame(format!(
            "{}: Data, WoW-Executable oder Interface/AddOns fehlt", root.display()
        )));
    }
    // Only accept known Forever client folder names unless the user has supplied
    // an explicit custom installation with a recognizable Forever TOC.
    if leaf != "_forever_" && leaf != "_classic_beta_" {
        let manifests = toc::scan_installed(&addons, None).unwrap_or_default();
        if !manifests.iter().any(|m| m.toc.forever_suffix) {
            return Err(AnvilError::InvalidGame(
                "Keine eindeutige Forever-Installation. Bitte _forever_ auswählen".into()
            ));
        }
    }
    let interface = infer_interface(&root);
    Ok(GameInstallation {
        id: root.to_string_lossy().into_owned(),
        label: format!("Forever ({leaf})"),
        path: root,
        interface,
    })
}

/// Prefer a client build marker over hard-coded interface constants.
/// WoW 1.60.1 => 16001; an unknown future format results in None.
pub fn infer_interface(root: &Path) -> Option<u32> {
    let mut markers = vec![root.join(".build.info")];
    if let Some(parent) = root.parent() { markers.push(parent.join(".build.info")); }
    for marker in markers {
        if let Ok(content) = fs::read_to_string(marker) {
            for line in content.lines() {
                for token in line.split(['|',' ','\t']) {
                    let digits: Vec<u32> = token.split('.').take(3)
                        .map(|s| s.parse::<u32>()).collect::<Option<Vec<_>>>()
                        .unwrap_or_default();
                    if digits.len() == 3 && digits[0] == 1 && digits[1] >= 50 && digits[2] < 100 {
                        return Some(digits[0]*10000 + digits[1]*100 + digits[2]);
                    }
                }
            }
        }
    }
    None
}

pub fn scan_candidates() -> Vec<GameInstallation> {
    let Some(home) = dirs::home_dir() else { return Vec::new() };
    let mut roots = vec![
        home.join("Games"),
        home.join(".wine/drive_c"),
        home.join(".local/share/lutris"),
        home.join(".steam/steam/steamapps/compatdata"),
        home.join(".local/share/Steam/steamapps/compatdata"),
    ];
    // Steam's libraryfolders.vdf: look only at declared libraries.
    for steam in [home.join(".steam/steam"), home.join(".local/share/Steam")] {
        if let Ok(vdf) = fs::read_to_string(steam.join("steamapps/libraryfolders.vdf")) {
            for line in vdf.lines() {
                if line.contains("\"path\"") {
                    if let Some(path) = line.split('"').nth(3) {
                        roots.push(PathBuf::from(path).join("steamapps/compatdata"));
                    }
                }
            }
        }
    }
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        bounded_scan(&root, 0, &mut 0usize, &mut |path| {
            if let Ok(game) = validate(path) {
                if seen.insert(game.id.clone()) { result.push(game); }
            }
        });
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

fn bounded_scan(root: &Path, depth: u8, visited: &mut usize, found: &mut impl FnMut(&Path)) {
    const MAX_DEPTH: u8 = 11;
    const MAX_FOLDERS: usize = 7000;
    if depth > MAX_DEPTH || *visited >= MAX_FOLDERS || !root.is_dir() || root.is_symlink() {
        return;
    }
    *visited += 1;
    if matches!(root.file_name().and_then(|n| n.to_str()), Some("_forever_" | "_classic_beta_")) {
        found(root);
        return;
    }
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else { continue };
        if !kind.is_dir() || kind.is_symlink() { continue; }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), "node_modules" | ".git" | "cache" | "Cache" | "logs" | "Logs" | "Downloads" | "shadercache" | "WTF" | "AddOns") {
            continue;
        }
        bounded_scan(&entry.path(), depth + 1, visited, found);
        if *visited >= MAX_FOLDERS { break; }
    }
}

/// A conservative Linux process check. The UI prevents writes if any WoW
/// process is found. This never kills or signals the game.
pub fn wow_running() -> bool {
    let Ok(entries) = fs::read_dir("/proc") else { return false };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) { continue; }
        let Ok(cmdline) = fs::read(entry.path().join("cmdline")) else { continue };
        let name = String::from_utf8_lossy(&cmdline).to_ascii_lowercase();
        if name.contains("wow.exe") || name.contains("wowclassic.exe")
            || name.contains("wowforever.exe") || name.contains("world of warcraft.exe") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verify_manual_install_and_build_info() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("_forever_");
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::write(temp.path().join(".build.info"), "Version!STRING:0\n1.60.1.70685\n").unwrap();
        let game = validate(&root).unwrap();
        assert_eq!(game.interface, Some(16001));
    }
    #[test]
    fn reject_unrelated_folder() {
        let temp = tempfile::tempdir().unwrap();
        assert!(validate(temp.path()).is_err());
    }
}
