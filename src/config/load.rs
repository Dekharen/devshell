use crate::config::schema::Config;
use crate::error::DevshellError;
use crate::fs;
use std::path::PathBuf;

pub fn load_config(name: Option<&str>) -> Result<Config, DevshellError> {
    if let Some(name) = name {
        load_named_config(name)
    } else {
        load_local_or_default_config()
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

fn load_local_or_default_config() -> Result<Config, DevshellError> {
    let local_configs = find_local_configs()?;

    match local_configs.len() {
        0 => load_default_config(),
        1 => load_config_from_path(&local_configs[0]),
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

    for entry in std::fs::read_dir(current_dir)? {
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
        return Err(DevshellError::ConfigNotFound(format!(
            "Default config not found at {}",
            default_path.display()
        )));
    }

    load_config_from_path(&default_path)
}

fn load_config_from_path(path: &PathBuf) -> Result<Config, DevshellError> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
