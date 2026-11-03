//! Defensive extraction of untrusted addon ZIP archives. This module never
//! extracts over an existing installation and does not follow archive symlinks.
use crate::error::{AnvilError, Result};
use crate::toc;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

pub const MAX_ARCHIVE_SIZE: u64 = 128 * 1024 * 1024;
pub const MAX_EXPANDED_SIZE: u64 = 384 * 1024 * 1024;
pub const MAX_FILES: usize = 12_000;

#[derive(Debug)]
pub struct PackageFolders {
    pub folders: Vec<(String, PathBuf)>,
}

/// Validate all entries first; if one is malicious, extract nothing.
pub fn extract_and_inspect(
    zip_path: &Path,
    dest: &Path,
    interface: Option<u32>,
    require_confirmed_forever: bool,
) -> Result<PackageFolders> {
    let zip_file = File::open(zip_path)?;
    if zip_file.metadata()?.len() > MAX_ARCHIVE_SIZE {
        return Err(AnvilError::TooLarge);
    }
    let mut archive = zip::ZipArchive::new(zip_file)?;
    if archive.len() > MAX_FILES {
        return Err(AnvilError::TooLarge);
    }
    let mut total = 0_u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let raw = entry.name();
        let path = entry
            .enclosed_name()
            .ok_or_else(|| AnvilError::UnsafePath(raw.into()))?;
        if raw.contains('\\')
            || raw.contains('\0')
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(AnvilError::UnsafePath(raw.into()));
        }
        if matches!(entry.unix_mode(), Some(mode) if mode & 0o170000 == 0o120000) {
            return Err(AnvilError::UnsafePath(format!("Symlink im Archiv: {raw}")));
        }
        total = total
            .checked_add(entry.size())
            .ok_or(AnvilError::TooLarge)?;
        if total > MAX_EXPANDED_SIZE {
            return Err(AnvilError::TooLarge);
        }
    }
    fs::create_dir_all(dest)?;
    let mut actual_total = 0_u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| AnvilError::UnsafePath(entry.name().into()))?
            .to_path_buf();
        let output = dest.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&output)?;
        let remaining = MAX_EXPANDED_SIZE - actual_total;
        let copied = io::copy(&mut entry.by_ref().take(remaining + 1), &mut file)?;
        actual_total += copied;
        if actual_total > MAX_EXPANDED_SIZE {
            return Err(AnvilError::TooLarge);
        }
        file.flush()?;
    }
    let mut result = Vec::new();
    find_addon_folders(dest, dest, 0, &mut result)?;
    result.sort_by(|a, b| a.0.cmp(&b.0));
    if result.is_empty() {
        return Err(AnvilError::InvalidOperation(
            "ZIP enthält keine gültigen Addon-Verzeichnisse mit passenden .toc-Dateien".into(),
        ));
    }
    let mut validated = Vec::new();
    for (name, path) in result {
        let Some((toc_path, dedicated)) = toc::preferred_toc(&path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&fs::read(toc_path)?).to_string();
        let meta = toc::parse(&text, dedicated);
        match toc::compatibility(&meta, interface) {
            Some(false) => return Err(AnvilError::Incompatible),
            Some(true) => {}
            None => {
                if require_confirmed_forever {
                    return Err(AnvilError::Incompatible);
                }
            }
        }
        validated.push((name, path));
    }
    Ok(PackageFolders { folders: validated })
}

fn find_addon_folders(
    root: &Path,
    dir: &Path,
    depth: u8,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<()> {
    if depth > 4 {
        return Ok(());
    }
    if dir != root {
        if toc::preferred_toc(dir).is_some() {
            let name = dir.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if !valid_folder_name(name) {
                return Err(AnvilError::UnsafePath(name.into()));
            }
            if out.iter().any(|(n, _)| n == name) {
                return Err(AnvilError::Conflict(format!("Doppelter Ordner {name}")));
            }
            out.push((name.into(), dir.to_owned()));
            return Ok(());
        }
    }
    for child in fs::read_dir(dir)? {
        let child = child?;
        if child.file_type()?.is_dir() && !child.path().is_symlink() {
            find_addon_folders(root, &child.path(), depth + 1, out)?;
        }
    }
    Ok(())
}

pub fn valid_folder_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(".addon-anvil")
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && !name.contains(':')
        && Path::new(name).components().count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        for &(name, bytes) in files {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    #[test]
    fn extracts_nested_multi_folder_package() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("pack.zip");
        make_zip(
            &file,
            &[
                ("Package/A/A_Forever.toc", b"## Title: A"),
                ("Package/B/B_Camelot.toc", b"## Title: B"),
            ],
        );
        let dest = tmp.path().join("out");
        let parsed = extract_and_inspect(&file, &dest, None, true).unwrap();
        assert_eq!(parsed.folders.len(), 2);
    }
    #[test]
    fn rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("evil.zip");
        make_zip(&file, &[("../evil.txt", b"danger")]);
        assert!(extract_and_inspect(&file, &tmp.path().join("out"), None, false).is_err());
        assert!(!tmp.path().join("evil.txt").exists());
    }
    #[test]
    fn refuses_wrong_client_interface() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("retail.zip");
        make_zip(&file, &[("Wrong/Wrong.toc", b"## Interface: 120000")]);
        assert!(matches!(
            extract_and_inspect(&file, &tmp.path().join("out"), Some(16001), true),
            Err(AnvilError::Incompatible)
        ));
    }
    #[test]
    fn only_expected_folder_names() {
        assert!(!valid_folder_name("../WTF"));
        assert!(!valid_folder_name("a\\b"));
        assert!(!valid_folder_name(".addon-anvil-stage"));
        assert!(valid_folder_name("QuestieDB"));
    }
}
