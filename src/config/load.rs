use crate::config::schema::Config;
use crate::error::{DevshellError, IoErrorContext};
use crate::fs;
use std::path::PathBuf;

pub fn load_config_with_source(name: Option<&str>) -> Result<(Config, bool), DevshellError> {
    if let Some(name) = name {
        let config = load_named_config(name)?;
        Ok((config, false)) // Named configs are not local
    } else {
        let (config, is_local) = load_local_or_default_config()?;
        Ok((config, is_local)) // Use the actual source flag
    }
}

fn load_named_config(name: &str) -> Result<Config, DevshellError> {
    let config_path = fs::get_config_dir().join(format!("{}.devshell.toml", name));

    if !config_path.exists() {
        return Err(DevshellError::ConfigNotFound(format!(
            "Named config '{}' not found at {}",
            name,
            config_path.display()
        )));
    }

    load_config_from_path(&config_path)
}

fn load_local_or_default_config() -> Result<(Config, bool), DevshellError> {
    let local_configs = find_local_configs()?;

    match local_configs.len() {
        0 => {
            let config = load_default_config()?;
            Ok((config, false)) // Default config is not local
        }
        1 => {
            let config = load_config_from_path(&local_configs[0])?;
            Ok((config, true)) // Local config is local
        }
        _ => Err(DevshellError::MultipleConfigs(
            local_configs
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        )),
    }
}

fn find_local_configs() -> Result<Vec<PathBuf>, DevshellError> {
    let current_dir = std::env::current_dir()?;
    let mut configs = Vec::new();

    for entry in
        std::fs::read_dir(current_dir).with_context("Reading current directory for config files")?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".devshell.toml") {
                    configs.push(path);
                }
            }
        }
    }

    Ok(configs)
}

fn load_default_config() -> Result<Config, DevshellError> {
    let default_path = fs::get_config_dir().join("default.devshell.toml");

    if !default_path.exists() {
        create_default_config(&default_path)?;
    }

    load_config_from_path(&default_path)
}

fn create_default_config(path: &std::path::Path) -> Result<(), DevshellError> {
    use crate::fragments::embedded;

    let default_config_content = embedded::get_default_config();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context_and_file(
            "Creating config directory for default config",
            &parent.to_string_lossy(),
        )?;
    }

    std::fs::write(path, default_config_content)
        .with_context_and_file("Creating default config file", &path.to_string_lossy())?;

    Ok(())
}

fn load_config_from_path(path: &PathBuf) -> Result<Config, DevshellError> {
    let content = std::fs::read_to_string(path)
        .with_context_and_file("Reading config file", &path.to_string_lossy())?;
    let config: Config = toml::from_str(&content)?;
    println!("Configuration : {config:?}");
    Ok(config)
}
