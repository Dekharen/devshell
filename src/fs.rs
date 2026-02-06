// Filesystem operations
use crate::error::{DevshellError, IoErrorContext};
use std::path::PathBuf;

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("devshell")
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("devshell")
}

pub fn get_fragments_dir() -> PathBuf {
    get_data_dir().join("fragments")
}

pub fn ensure_directories_exist() -> Result<(), DevshellError> {
    let config_dir = get_config_dir();
    let fragments_dir = get_fragments_dir();

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .with_context_and_file("Creating config directory", &config_dir.to_string_lossy())?;
    }

    if !fragments_dir.exists() {
        std::fs::create_dir_all(&fragments_dir).with_context_and_file(
            "Creating fragments directory",
            &fragments_dir.to_string_lossy(),
        )?;
    }

    Ok(())
}

pub fn ensure_dir_exists(path: &std::path::Path) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn discover_disk_fragments() -> Result<Vec<String>, DevshellError> {
    let fragments_dir = get_fragments_dir();
    let mut fragments = Vec::new();

    if !fragments_dir.exists() {
        return Ok(fragments);
    }

    for entry in std::fs::read_dir(&fragments_dir).with_context_and_file(
        "Reading fragments directory",
        &fragments_dir.to_string_lossy(),
    )? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("docker") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                fragments.push(stem.to_string());
            }
        }
    }

    Ok(fragments)
}
