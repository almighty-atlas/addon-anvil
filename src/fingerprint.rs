//! File-level ownership fingerprints. The hash covers path names, directory
//! boundaries and the contents of every regular file. No symlink is followed.
use crate::archive::valid_folder_name;
use crate::error::{AnvilError, Result};
use crate::model::ManagedAddon;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const MAX_ITEMS: usize = 12_000;
const MAX_BYTES: u64 = 384 * 1024 * 1024;

pub fn folder_fingerprint(root: &Path) -> Result<String> {
    fn visit(
        root: &Path,
        path: &Path,
        digest: &mut Sha256,
        items: &mut usize,
        bytes: &mut u64,
    ) -> Result<()> {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
            return Err(AnvilError::UnsafePath(path.display().to_string()));
        }
        *items += 1;
        if *items > MAX_ITEMS {
            return Err(AnvilError::TooLarge);
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| AnvilError::UnsafePath(path.display().to_string()))?;
        let name = relative.as_os_str().as_encoded_bytes();
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name);
        digest.update([u8::from(meta.is_file())]);
        if meta.is_dir() {
            let mut children: Vec<_> = fs::read_dir(path)?.collect::<std::io::Result<_>>()?;
            children.sort_by_key(|item| item.file_name());
            for child in children {
                visit(root, &child.path(), digest, items, bytes)?;
            }
        } else {
            let mut reader = File::open(path)?;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                *bytes = bytes.checked_add(read as u64).ok_or(AnvilError::TooLarge)?;
                if *bytes > MAX_BYTES {
                    return Err(AnvilError::TooLarge);
                }
                digest.update(&buffer[..read]);
            }
        }
        Ok(())
    }
    let mut digest = Sha256::new();
    let mut entries = 0;
    let mut bytes = 0;
    visit(root, root, &mut digest, &mut entries, &mut bytes)?;
    Ok(format!("{:x}", digest.finalize()))
}

pub fn verify_owned(addons: &Path, addon: &ManagedAddon) -> Result<()> {
    for folder in &addon.folders {
        if !valid_folder_name(folder) {
            return Err(AnvilError::UnsafePath(folder.clone()));
        }
        let path = addons.join(folder);
        let Some(expected) = addon.folder_hashes.get(folder) else {
            return Err(AnvilError::Conflict(format!(
                "Keine gesicherte Dateizuordnung für {folder}; keine automatische Löschung"
            )));
        };
        if !path.is_dir() || folder_fingerprint(&path)? != *expected {
            return Err(AnvilError::Conflict(format!(
                "Ordner {folder} wurde extern verändert; bitte manuell prüfen"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_detects_external_changes() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("MyAddon");
        fs::create_dir_all(&folder).unwrap();
        let file = folder.join("MyAddon_Forever.toc");
        fs::write(&file, "old").unwrap();
        let first = folder_fingerprint(&folder).unwrap();
        fs::write(&file, "new").unwrap();
        assert_ne!(first, folder_fingerprint(&folder).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn fingerprint_rejects_symlink_nodes() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("MyAddon");
        fs::create_dir_all(&folder).unwrap();
        std::os::unix::fs::symlink(temp.path(), folder.join("escape")).unwrap();
        assert!(matches!(
            folder_fingerprint(&folder),
            Err(AnvilError::UnsafePath(_))
        ));
    }
}
