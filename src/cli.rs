use crate::config::load;
use crate::docker::{build, run};
use crate::error::DevshellError;
use crate::fragments::resolve;
use crate::fs;
use crate::util;
use clap::{Parser, Subcommand};
use std::process::exit;

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
            let config = load::load_config(name.as_deref())?;
            let image_name = build::build_image(&config.name, &config.stages)?;
            run::run_container(&config, &image_name)?;
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
    }
    Ok(())
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
    crate::fs::ensure_dir_exists(output_path.parent().unwrap())?;
    std::fs::write(&output_path, content)?;

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
