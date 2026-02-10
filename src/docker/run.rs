use super::container;
use crate::config::schema::Config;
use crate::error::DevshellError;
use std::path::Path;
use std::process::Command;

pub fn run_container(
    config: &Config,
    image_name: &str,
    container_name: &str,
) -> Result<(), DevshellError> {
    // Check if container already exists
    if container::container_exists(container_name)? {
        let is_running = container::is_container_running(container_name)?;

        if is_running {
            // Container is running, prompt user
            let action = container::prompt_container_action(container_name)?;
            match action.as_str() {
                "attach" => {
                    if let Some(attach_cmd) = &config.attach_command {
                        container::attach_to_container(container_name, attach_cmd)?;
                    } else {
                        return Err(DevshellError::DockerError(
                            "Cannot attach to container: attach_command not set in config"
                                .to_string(),
                        ));
                    }
                }
                "replace" => {
                    // Remove existing container and recreate
                    Command::new("docker")
                        .args(&["rm", "-f", container_name])
                        .output()
                        .map_err(|e| DevshellError::IoErrorWithContext {
                            error: e,
                            context: "Removing existing container".to_string(),
                            file_path: None,
                        })?;

                    container::run_attached_container(config, image_name, container_name)?;
                }
                "quit" => {
                    println!("Operation cancelled.");
                    return Ok(());
                }
                _ => unreachable!(),
            }
        } else {
            // Container exists but is stopped, just start it
            container::start_container(container_name)?;

            if let Some(attach_cmd) = &config.attach_command {
                container::attach_to_container(container_name, attach_cmd)?;
            }
        }
    } else {
        // Container doesn't exist, create new one
        if config.attach_command.is_some() {
            // Long-lived container with attach command
            container::run_attached_container(config, image_name, container_name)?;
        } else {
            // Standard one-shot container (existing logic)
            run_simple_container(config, image_name)?;
        }
    }

    Ok(())
}

fn run_simple_container(config: &Config, image_name: &str) -> Result<(), DevshellError> {
    let mut args = vec!["run".to_string()];

    args.extend(config.docker_args.clone());

    for volume in &config.volumes {
        let host_path = if volume.host == "." {
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
        } else {
            Path::new(&volume.host).to_path_buf()
        };

        args.push("-v".to_string());
        args.push(format!(
            "{}:{}:{}",
            host_path.display(),
            volume.container,
            volume.mode
        ));
    }

    args.push("-it".to_string());
    args.push(image_name.to_string());

    let mut child = Command::new("docker").args(&args).spawn().map_err(|e| {
        DevshellError::IoErrorWithContext {
            error: e,
            context: "Spawning docker run command".to_string(),
            file_path: None,
        }
    })?;

    let status = child
        .wait()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Waiting for docker run command".to_string(),
            file_path: None,
        })?;

    if !status.success() {
        return Err(DevshellError::DockerError(format!(
            "Docker run exited with non-zero status: {:?}",
            status
        )));
    }

    Ok(())
}
