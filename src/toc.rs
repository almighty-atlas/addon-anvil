use crate::error::Result;
use crate::model::{ScannedAddon, TocMetadata};
use std::fs;
use std::path::Path;

fn split_names(input: &str) -> Vec<String> {
    input
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Remove WoW's inline color escapes from user-visible TOC metadata.
/// Only complete |cAARRGGBB and |r codes are recognized; malformed codes
/// remain visible instead of accidentally swallowing a title's contents.
pub fn strip_wow_colors(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let rest = &input[index..];
        let bytes = rest.as_bytes();
        if bytes[0] == b'|' && bytes.len() >= 2 {
            match bytes[1] {
                b'c' | b'C'
                    if bytes.len() >= 10 && bytes[2..10].iter().all(u8::is_ascii_hexdigit) =>
                {
                    index += 10;
                    continue;
                }
                b'r' | b'R' => {
                    index += 2;
                    continue;
                }
                b'|' => {
                    cleaned.push('|');
                    index += 2;
                    continue;
                }
                _ => {}
            }
        }
        let next = rest.chars().next().expect("valid char boundary");
        cleaned.push(next);
        index += next.len_utf8();
    }
    cleaned
}

pub fn parse(content: &str, forever_suffix: bool) -> TocMetadata {
    let mut metadata = TocMetadata {
        forever_suffix,
        ..TocMetadata::default()
    };
    for line in content.trim_start_matches('\u{feff}').lines() {
        let Some(rest) = line.trim().strip_prefix("##") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match name.trim().to_ascii_lowercase().as_str() {
            "title" => metadata.title = Some(strip_wow_colors(&value)),
            "notes" => metadata.notes = Some(strip_wow_colors(&value)),
            "version" => metadata.version = Some(strip_wow_colors(&value)),
            "interface" => {
                metadata.interfaces.extend(
                    value
                        .split([',', ' ', '\t'])
                        .filter_map(|s| s.trim().parse::<u32>().ok()),
                );
            }
            "dependencies" | "deps" | "requireddeps" => metadata.dependencies = split_names(&value),
            "optionaldeps" => metadata.optional_dependencies = split_names(&value),
            "x-wowi-id" => metadata.wowi_id = Some(value),
            "x-website" => metadata.website = Some(value),
            _ => {}
        }
    }
    metadata
}

/// A specific Forever/Camelot manifest takes precedence over a shared Mainline TOC.
/// A different-flavour TOC must never be mistaken for a compatible addon.
pub fn preferred_toc(folder: &Path) -> Option<(std::path::PathBuf, bool)> {
    let name = folder.file_name()?.to_str()?;
    for (suffix, dedicated) in [
        ("_Forever.toc", true),
        ("_Camelot.toc", true),
        ("_Mainline.toc", false),
        (".toc", false),
    ] {
        let candidate = folder.join(format!("{name}{suffix}"));
        if candidate.is_file() && !candidate.is_symlink() {
            return Some((candidate, dedicated));
        }
    }
    None
}

pub fn compatibility(metadata: &TocMetadata, current_interface: Option<u32>) -> Option<bool> {
    if metadata.forever_suffix {
        return Some(true);
    }
    if metadata.interfaces.is_empty() {
        return None;
    }
    current_interface.map(|version| metadata.interfaces.contains(&version))
}

pub fn scan_installed(addons: &Path, current: Option<u32>) -> Result<Vec<ScannedAddon>> {
    if !addons.exists() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in fs::read_dir(addons)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || entry.path().is_symlink() {
            continue;
        }
        if let Some((toc, dedicated)) = preferred_toc(&entry.path()) {
            let bytes = fs::read(&toc)?;
            let text = String::from_utf8_lossy(&bytes);
            let metadata = parse(&text, dedicated);
            let compatible = compatibility(&metadata, current);
            found.push(ScannedAddon {
                folder: entry.file_name().to_string_lossy().into_owned(),
                toc: metadata,
                compatible,
            });
        }
    }
    found.sort_by_key(|a| a.folder.to_lowercase());
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_metadata_and_multiple_versions() {
        let toc = parse(
            "\u{feff}## Title: Hallo\n## Interface: 16001, 16002\n## Version: 2.4\n## OptionalDeps: LibA, LibB\n## X-WoWI-ID: 1234\n",
            false,
        );
        assert_eq!(toc.interfaces, vec![16001, 16002]);
        assert_eq!(toc.optional_dependencies, vec!["LibA", "LibB"]);
        assert_eq!(toc.wowi_id.as_deref(), Some("1234"));
        assert_eq!(compatibility(&toc, Some(16001)), Some(true));
        assert_eq!(compatibility(&toc, Some(15000)), Some(false));
        assert_eq!(compatibility(&toc, None), None);
    }
    #[test]
    fn strips_wow_title_and_notes_color_codes() {
        let toc = parse(
            "## Title: AtlasLoot Continued |cFF22B14C[Core]|r\n## Notes: Grüße |cff0099FFWelt|r!\n## Version: |cffaabbcc1.0|r\n",
            true,
        );
        assert_eq!(toc.title.as_deref(), Some("AtlasLoot Continued [Core]"));
        assert_eq!(toc.notes.as_deref(), Some("Grüße Welt!"));
        assert_eq!(toc.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn leaves_malformed_escapes_and_unicode_intact() {
        assert_eq!(strip_wow_colors("A |cFF00 fehlt"), "A |cFF00 fehlt");
        assert_eq!(strip_wow_colors("A || B |x C"), "A | B |x C");
        assert_eq!(strip_wow_colors("|cff12aa99日本語|r"), "日本語");
        assert_eq!(strip_wow_colors("No markup"), "No markup");
    }

    #[test]
    fn suffix_beats_generic() {
        let dir = tempfile::tempdir().unwrap();
        let addon = dir.path().join("TestAddon");
        fs::create_dir_all(&addon).unwrap();
        fs::write(addon.join("TestAddon.toc"), "## Interface: 120000").unwrap();
        fs::write(addon.join("TestAddon_Forever.toc"), "## Title: Forever").unwrap();
        let scanned = scan_installed(dir.path(), None).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].compatible, Some(true));
    }
}
