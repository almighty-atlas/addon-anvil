use crate::archive::{self, PackageFolders};
use crate::config::Paths;
use crate::error::{AnvilError, Result};
use crate::model::{AppConfig, InstallRequest, ManagedAddon, ProviderKind};
use crate::wow;
use std::fs;
use std::path::{Path, PathBuf};

fn game_and_addons(cfg: &AppConfig, id: &str) -> Result<PathBuf> {
    if wow::wow_running() {
        return Err(AnvilError::GameRunning);
    }
    let game = cfg
        .installations
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| AnvilError::InvalidGame("Installation fehlt in Konfiguration".into()))?;
    // Revalidate paths just before any changes, including after app startup.
    let validated = wow::validate(&game.path)?;
    let addons = validated.addons_dir();
    let interface = addons
        .parent()
        .ok_or_else(|| AnvilError::UnsafePath("Interface".into()))?;
    if interface.is_symlink() || addons.is_symlink() {
        return Err(AnvilError::UnsafePath("Symlink in Interface/AddOns".into()));
    }
    fs::create_dir_all(&addons)?;
    let root = addons.canonicalize()?;
    if !root.starts_with(&validated.path) {
        return Err(AnvilError::UnsafePath(root.display().to_string()));
    }
    Ok(root)
}

fn guarded_dir(root: &Path, folder: &str) -> Result<PathBuf> {
    if !archive::valid_folder_name(folder) {
        return Err(AnvilError::UnsafePath(folder.into()));
    }
    let path = root.join(folder);
    if path.is_symlink() {
        return Err(AnvilError::UnsafePath(format!(
            "Symlink: {}",
            path.display()
        )));
    }
    Ok(path)
}

/// Install or update a project as one unit. All owned folders are moved to an
/// on-filesystem backup before the new folders are activated. Any error before
/// the config commit restores the original folders.
pub fn install_zip(
    paths: &Paths,
    cfg: &AppConfig,
    install_id: &str,
    archive_path: &Path,
    request: InstallRequest,
) -> Result<AppConfig> {
    let addons = game_and_addons(cfg, install_id)?;
    let current_interface = cfg
        .installations
        .iter()
        .find(|g| g.id == install_id)
        .and_then(|g| g.interface);
    let temp = tempfile::Builder::new()
        .prefix(".addon-anvil-stage-")
        .tempdir_in(&addons)?;
    let unpacked = temp.path().join("incoming");
    let PackageFolders { folders } = archive::extract_and_inspect(
        archive_path,
        &unpacked,
        current_interface,
        request.provider != ProviderKind::Local,
    )?;
    let old = cfg.managed.iter().find(|m| {
        m.installation_id == install_id
            && m.provider == request.provider
            && m.project_id == request.project_id
    });
    let new_folders: Vec<String> = folders.iter().map(|(name, _)| name.clone()).collect();
    for folder in &new_folders {
        let dest = guarded_dir(&addons, folder)?;
        if let Some(other) = cfg.owned_folder(install_id, folder) {
            if old
                .map(|p| p.project_id != other.project_id || p.provider != other.provider)
                .unwrap_or(true)
            {
                return Err(AnvilError::Conflict(folder.clone()));
            }
        } else if dest.exists() {
            return Err(AnvilError::Conflict(folder.clone()));
        }
    }
    let old_names = old.map(|item| item.folders.clone()).unwrap_or_default();
    let backup = temp.path().join("backup");
    fs::create_dir_all(&backup)?;
    let mut moved_old: Vec<String> = Vec::new();
    let mut moved_new: Vec<String> = Vec::new();

    let transaction = (|| -> Result<AppConfig> {
        for folder in &old_names {
            let from = guarded_dir(&addons, folder)?;
            if from.exists() {
                fs::rename(&from, guarded_dir(&backup, folder)?)?;
                moved_old.push(folder.clone());
            }
        }
        for (folder, unpacked_path) in &folders {
            fs::rename(unpacked_path, guarded_dir(&addons, folder)?)?;
            moved_new.push(folder.clone());
        }
        let mut updated = cfg.clone();
        updated.managed.retain(|m| {
            !(m.installation_id == install_id
                && m.provider == request.provider
                && m.project_id == request.project_id)
        });
        updated.managed.push(ManagedAddon {
            installation_id: install_id.to_string(),
            provider: request.provider,
            project_id: request.project_id,
            name: request.name,
            release_id: request.release_id,
            folders: new_folders,
            project_url: request.project_url,
        });
        paths.save(&updated)?;
        Ok(updated)
    })();
    if transaction.is_err() {
        // Reverse new file placement, then restore the old installation.
        for folder in moved_new.iter().rev() {
            if let Ok(path) = guarded_dir(&addons, folder) {
                let _ = fs::remove_dir_all(path);
            }
        }
        for folder in moved_old.iter().rev() {
            if let (Ok(from), Ok(to)) = (guarded_dir(&backup, folder), guarded_dir(&addons, folder))
            {
                let _ = fs::rename(from, to);
            }
        }
    }
    transaction
}

/// Uninstalls ONLY folders recorded as owned by this project and leaves WTF
/// entirely untouched. A failed config save restores the moved folders.
pub fn uninstall(
    paths: &Paths,
    cfg: &AppConfig,
    install_id: &str,
    provider: ProviderKind,
    project_id: &str,
) -> Result<AppConfig> {
    let addons = game_and_addons(cfg, install_id)?;
    let old = cfg
        .managed
        .iter()
        .find(|m| {
            m.installation_id == install_id && m.provider == provider && m.project_id == project_id
        })
        .ok_or_else(|| AnvilError::InvalidOperation("Addon ist nicht verwaltet".into()))?;
    let temp = tempfile::Builder::new()
        .prefix(".addon-anvil-remove-")
        .tempdir_in(&addons)?;
    let mut moved = Vec::new();
    let transaction = (|| -> Result<AppConfig> {
        for folder in &old.folders {
            let target = guarded_dir(&addons, folder)?;
            if target.exists() {
                if !target.is_dir() {
                    return Err(AnvilError::Conflict(folder.clone()));
                }
                fs::rename(&target, guarded_dir(temp.path(), folder)?)?;
                moved.push(folder.clone());
            }
        }
        let mut updated = cfg.clone();
        updated.managed.retain(|m| {
            !(m.installation_id == install_id
                && m.provider == provider
                && m.project_id == project_id)
        });
        paths.save(&updated)?;
        Ok(updated)
    })();
    if transaction.is_err() {
        for folder in moved.iter().rev() {
            if let (Ok(from), Ok(to)) = (
                guarded_dir(temp.path(), folder),
                guarded_dir(&addons, folder),
            ) {
                let _ = fs::rename(from, to);
            }
        }
    }
    transaction
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn test_game() -> (tempfile::TempDir, Paths, AppConfig, String) {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("_forever_");
        fs::create_dir_all(game.join("Interface/AddOns")).unwrap();
        let game = wow::validate(&game).unwrap();
        let id = game.id.clone();
        let paths = Paths {
            config: dir.path().join("config"),
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        };
        let cfg = AppConfig {
            installations: vec![game],
            selected_installation: Some(id.clone()),
            ..AppConfig::default()
        };
        (dir, paths, cfg, id)
    }

    fn zip_addon(path: &Path, value: &str) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        zip.start_file("Demo/Demo_Forever.toc", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(value.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    fn request(version: &str) -> InstallRequest {
        InstallRequest {
            provider: ProviderKind::Local,
            project_id: "fixture".into(),
            name: "Demo".into(),
            release_id: version.into(),
            project_url: String::new(),
        }
    }

    #[test]
    fn install_update_remove_and_preserve_wtf() {
        let (tmp, paths, cfg, id) = test_game();
        let zip = tmp.path().join("addon.zip");
        zip_addon(&zip, "## Version: 1");
        let cfg = install_zip(&paths, &cfg, &id, &zip, request("1")).unwrap();
        assert_eq!(cfg.managed[0].folders, ["Demo"]);
        let target = cfg
            .selected()
            .unwrap()
            .addons_dir()
            .join("Demo/Demo_Forever.toc");
        assert_eq!(fs::read_to_string(&target).unwrap(), "## Version: 1");
        zip_addon(&zip, "## Version: 2");
        let cfg = install_zip(&paths, &cfg, &id, &zip, request("2")).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "## Version: 2");
        let wtf = cfg.selected().unwrap().path.join("WTF/saved.txt");
        fs::create_dir_all(wtf.parent().unwrap()).unwrap();
        fs::write(&wtf, "keep").unwrap();
        let cfg = uninstall(&paths, &cfg, &id, ProviderKind::Local, "fixture").unwrap();
        assert!(cfg.managed.is_empty());
        assert!(!target.exists());
        assert_eq!(fs::read_to_string(wtf).unwrap(), "keep");
    }

    #[test]
    fn never_clobber_unmanaged_files() {
        let (tmp, paths, cfg, id) = test_game();
        let addons = cfg.selected().unwrap().addons_dir();
        fs::create_dir_all(addons.join("Demo")).unwrap();
        fs::write(addons.join("Demo/Demo_Forever.toc"), "original").unwrap();
        let zip = tmp.path().join("addon.zip");
        zip_addon(&zip, "new");
        assert!(matches!(
            install_zip(&paths, &cfg, &id, &zip, request("1")),
            Err(AnvilError::Conflict(_))
        ));
        assert_eq!(
            fs::read_to_string(addons.join("Demo/Demo_Forever.toc")).unwrap(),
            "original"
        );
    }

    #[test]
    fn failed_config_save_rolls_back_update() {
        let (tmp, paths, cfg, id) = test_game();
        let zip = tmp.path().join("addon.zip");
        zip_addon(&zip, "old");
        let cfg = install_zip(&paths, &cfg, &id, &zip, request("1")).unwrap();
        zip_addon(&zip, "new");
        // A regular file where a config directory should be forces failure.
        let blocked = tmp.path().join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        let paths = Paths {
            config: blocked,
            ..paths
        };
        assert!(install_zip(&paths, &cfg, &id, &zip, request("2")).is_err());
        assert_eq!(
            fs::read_to_string(
                cfg.selected()
                    .unwrap()
                    .addons_dir()
                    .join("Demo/Demo_Forever.toc")
            )
            .unwrap(),
            "old"
        );
    }

    #[cfg(unix)]
    #[test]
    fn reject_symlink_escape() {
        let (tmp, paths, cfg, id) = test_game();
        let addons = cfg.selected().unwrap().addons_dir();
        let external = tmp.path().join("elsewhere");
        fs::create_dir(&external).unwrap();
        std::os::unix::fs::symlink(&external, addons.join("Demo")).unwrap();
        let zip = tmp.path().join("addon.zip");
        zip_addon(&zip, "new");
        assert!(install_zip(&paths, &cfg, &id, &zip, request("1")).is_err());
        assert!(external.is_dir());
    }
}
