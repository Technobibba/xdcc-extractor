use crate::{config::Config, extractor};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

pub fn is_scan_command() -> bool {
    env::args().any(|arg| arg == "--scan" || arg == "scan")
}

pub fn run_from_args() -> Result<()> {
    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    print_scan(&config)
}

pub fn print_scan(config: &Config) -> Result<()> {
    println!("== XDCC Extractor Scan ==");
    println!();

    println!("Watch directory:");
    println!("  {}", config.watch.directory);
    println!();

    println!("Root-Archive erlaubt:");
    println!("  {}", config.watch.allow_root_archives);
    println!();

    let candidates = scan_candidates(config)?;

    println!("Gefundene Kandidaten:");
    println!("  {}", candidates.len());
    println!();

    if candidates.is_empty() {
        println!("Keine verarbeitbaren Releases gefunden.");
    } else {
        for candidate in candidates {
            println!("  - {}", candidate.display());
        }
    }

    println!();
    println!("Scan abgeschlossen. Es wurde nichts entpackt und nichts gelöscht.");

    Ok(())
}

fn scan_candidates(config: &Config) -> Result<Vec<PathBuf>> {
    let watch_dir = Path::new(&config.watch.directory);

    if !watch_dir.is_dir() {
        anyhow::bail!("Watch directory existiert nicht: {}", watch_dir.display());
    }

    let mut candidates = BTreeSet::new();

    for entry in fs::read_dir(watch_dir)
        .with_context(|| format!("Konnte Watch-Ordner nicht lesen: {}", watch_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if should_skip_entry(&name) {
            continue;
        }

        if path.is_dir() {
            if extractor::has_archive_start(&path)? {
                candidates.insert(path);
            }

            continue;
        }

        if path.is_file()
            && config.watch.allow_root_archives
            && extractor::is_archive_related_file(&path)
        {
            if let Some(target) = extractor::root_archive_target(&path) {
                if target.exists() && extractor::has_archive_start(&target)? {
                    candidates.insert(target);
                }
            }
        }
    }

    Ok(candidates.into_iter().collect())
}

fn should_skip_entry(name: &str) -> bool {
    matches!(
        name,
        "_extracted" | "_failed" | "_processing" | ".xdcc-worker"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_finds_folder_and_root_archive_candidates() {
        let dir = tempdir().expect("tempdir");

        let folder_release = dir.path().join("Folder.Release");
        fs::create_dir_all(&folder_release).expect("create folder release");
        fs::write(folder_release.join("Folder.Release.rar"), "rar").expect("write folder rar");

        let root_part1 = dir.path().join("Root.Release.part01.rar");
        let root_part2 = dir.path().join("Root.Release.part02.rar");
        fs::write(&root_part1, "part1").expect("write part1");
        fs::write(&root_part2, "part2").expect("write part2");

        let ignored_dir = dir.path().join("_extracted");
        fs::create_dir_all(&ignored_dir).expect("create ignored dir");
        fs::write(ignored_dir.join("Ignored.Release.rar"), "rar").expect("write ignored rar");

        let config_file = dir.path().join("config.toml");
        fs::write(
            &config_file,
            format!(
                r#"
[watch]
directory="{}"
allow_root_archives=true
"#,
                dir.path().display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let candidates = scan_candidates(&config).expect("scan");

        assert!(candidates.contains(&folder_release));
        assert!(candidates.contains(&root_part1));
        assert!(!candidates.contains(&ignored_dir));
    }

    #[test]
    fn scan_ignores_root_archives_when_disabled() {
        let dir = tempdir().expect("tempdir");

        let root_archive = dir.path().join("Root.Disabled.zip");
        fs::write(&root_archive, "zip").expect("write zip");

        let config_file = dir.path().join("config.toml");
        fs::write(
            &config_file,
            format!(
                r#"
[watch]
directory="{}"
allow_root_archives=false
"#,
                dir.path().display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let candidates = scan_candidates(&config).expect("scan");

        assert!(candidates.is_empty());
    }
}
