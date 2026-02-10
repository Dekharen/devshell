use crate::config::load;
use crate::docker::{build, container, run};
use crate::error::{DevshellError, IoErrorContext};
use crate::fragments::resolve;
use crate::fs;
use crate::util;
use clap::{Parser, Subcommand};
use std::process::{exit, Command};

#[derive(Parser)]
#[command(name = "devshell")]
#[command(about = "A lightweight CLI tool for composing and running developer containers")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a devshell container
    Run {
        /// Configuration name (optional)
        name: Option<String>,
    },
    /// Show a Dockerfile fragment
    Show {
        /// Fragment reference (e.g., 'base/debian' or '@base/debian')
        fragment: String,
    },
    /// Generate embedded fragments to disk
    Generate {
        /// Fragment reference or 'all' (e.g., '@base/debian' or 'all')
        target: String,
    },
    /// Check system and configuration
    Doctor,
    /// Attach to a running devshell container
    Attach {
        /// Container name or config name
        name: String,
    },
}

pub fn run() {
    let cli = Cli::parse();

    if let Err(e) = handle_command(cli.command) {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn handle_command(command: Commands) -> Result<(), DevshellError> {
    match command {
        Commands::Run { name } => {
            let (config, is_local) = load::load_config_with_source(name.as_deref())?;
            let image_name = build::build_image(&config.name, &config.stages)?;
            let container_name = container::get_container_name(&config.name, is_local);
            run::run_container(&config, &image_name, &container_name)?;
            build::cleanup_temp_files()?;
        }
        Commands::Show { fragment } => {
            let content = resolve::resolve_fragment(&fragment)?;
            println!("{}", content);
        }
        Commands::Generate { target } => {
            if target == "all" {
                generate_all_fragments()?;
            } else {
                generate_fragment(&target)?;
            }
        }
        Commands::Doctor => {
            util::run_doctor()?;
        }
        Commands::Attach { name } => {
            attach_to_container(&name)?;
        }
    }
    Ok(())
}

fn attach_to_container(name: &String) -> Result<(), DevshellError> {
    let running_containers = list_devshell_containers()?;

    let target_container = if running_containers.contains(name) {
        Some(name.to_string())
    } else {
        find_container_by_config_name(name)?
    };

    if let Some(container_name) = target_container {
        let (config, _) = load::load_config_with_source(Some(name))?;

        if let Some(attach_cmd) = config.attach_command {
            container::attach_to_container(&container_name, &attach_cmd)
        } else {
            let msg = format!(
                "Container '{}' exists but has no attach_command configured",
                container_name
            );
            return Err(DevshellError::DockerError(msg));
        }
    } else {
        let msg = format!("No running devshell container found matching '{}'", name);
        return Err(DevshellError::DockerError(msg));
    }
}

fn list_devshell_containers() -> Result<Vec<String>, DevshellError> {
    let output = Command::new("docker")
        .args(&["ps", "--format={{.Names}}"])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Listing running containers".to_string(),
            file_path: None,
        })?;

    let container_names = String::from_utf8_lossy(&output.stdout);
    Ok(container_names
        .lines()
        .filter(|name| name.starts_with("devshell-"))
        .map(|name| {
            name.trim_start_matches("devshell-")
                // .unwrap_or(name)
                .to_string()
        })
        .collect())
}

fn find_container_by_config_name(config_name: &str) -> Result<Option<String>, DevshellError> {
    let running_containers = list_devshell_containers()?;

    for container in &running_containers {
        if container.as_str() == config_name {
            return Ok(Some(container::get_container_name(config_name, false)));
        }
        if container.as_str() == format!("local-{}", config_name) {
            return Ok(Some(container::get_container_name(config_name, true)));
        }
    }
    Ok(None)
}

fn generate_fragment(fragment_ref: &str) -> Result<(), DevshellError> {
    if !fragment_ref.starts_with('@') {
        return Err(DevshellError::FragmentNotFound(
            "Only embedded fragments (starting with @) can be generated".to_string(),
        ));
    }

    let fragment_path = &fragment_ref[1..];
    let content = resolve::resolve_fragment(fragment_ref)?;

    let output_path = fs::get_fragments_dir().join(format!("{}.docker", fragment_path));
    crate::fs::ensure_dir_exists(output_path.parent().unwrap()).map_err(|e| {
        DevshellError::IoErrorWithContext {
            error: e,
            context: "Creating fragment directory".to_string(),
            file_path: Some(output_path.parent().unwrap().to_string_lossy().to_string()),
        }
    })?;
    std::fs::write(&output_path, content)
        .with_context_and_file("Writing fragment file", &output_path.to_string_lossy())?;

    println!("Generated fragment: {}", output_path.display());
    Ok(())
}

fn generate_all_fragments() -> Result<(), DevshellError> {
    let embedded_fragments = resolve::list_embedded_fragments();

    for fragment in &embedded_fragments {
        let fragment_ref = format!("@{}", fragment);
        generate_fragment(&fragment_ref)?;
    }

    println!("Generated {} embedded fragments", embedded_fragments.len());
    Ok(())
}
