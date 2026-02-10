use crate::config::schema::Config;
use crate::error::DevshellError;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn get_container_name(config_name: &str, is_local: bool) -> String {
    if is_local {
        format!("devshell-local-{}", config_name)
    } else {
        format!("devshell-{}", config_name)
    }
}

pub fn container_exists(container_name: &str) -> Result<bool, DevshellError> {
    let output = Command::new("docker")
        .args(&["inspect", "--format={{.State.Status}}", container_name])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Checking container status".to_string(),
            file_path: None,
        })?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(status != "no such container")
}

pub fn is_container_running(container_name: &str) -> Result<bool, DevshellError> {
    let output = Command::new("docker")
        .args(&["inspect", "--format={{.State.Status}}", container_name])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Checking if container is running".to_string(),
            file_path: None,
        })?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(status == "running")
}

pub fn start_container(container_name: &str) -> Result<(), DevshellError> {
    let output = Command::new("docker")
        .args(&["start", container_name])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Starting stopped container".to_string(),
            file_path: None,
        })?;

    if !output.status.success() {
        return Err(DevshellError::DockerError(format!(
            "Failed to start container: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

pub fn run_attached_container(
    config: &Config,
    image_name: &str,
    container_name: &str,
) -> Result<(), DevshellError> {
    // Start container in detached mode
    let mut run_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.to_string(),
    ];
    run_args.extend(config.docker_args.clone());

    // Add volumes
    for volume in &config.volumes {
        let host_path = if volume.host == "." {
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
        } else {
            Path::new(&volume.host).to_path_buf()
        };

        run_args.push("-v".to_string());
        run_args.push(format!(
            "{}:{}:{}",
            host_path.display(),
            volume.container,
            volume.mode
        ));
    }

    run_args.push(image_name.to_string());

    let output = Command::new("docker")
        .args(&run_args)
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Starting container in detached mode".to_string(),
            file_path: None,
        })?;

    if !output.status.success() {
        return Err(DevshellError::DockerError(format!(
            "Failed to start container: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Wait for container to be ready and running
    let mut retries = 10;
    while retries > 0 {
        if is_container_running(container_name)? {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        retries -= 1;
    }

    if !is_container_running(container_name)? {
        return Err(DevshellError::DockerError(format!(
            "Container failed to start within expected time"
        )));
    }

    // Additional pause for container initialization
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Attach immediately
    let attach_cmd = config.attach_command.as_deref().unwrap_or("/bin/bash");

    let mut child = Command::new("docker")
        .args(&["exec", "-it", container_name, attach_cmd])
        .spawn()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Attaching to container".to_string(),
            file_path: None,
        })?;

    let status = child
        .wait()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Waiting for attach command".to_string(),
            file_path: None,
        })?;

    if !status.success() {
        return Err(DevshellError::DockerError(format!(
            "Attach command failed: container exited with status {:?}",
            status
        )));
    }

    Ok(())
}

pub fn attach_to_container(
    container_name: &str,
    attach_command: &str,
) -> Result<(), DevshellError> {
    // Verify container is running before attaching
    if !is_container_running(container_name)? {
        return Err(DevshellError::DockerError(format!(
            "Cannot attach to container '{}': container is not running",
            container_name
        )));
    }

    let mut child = Command::new("docker")
        .args(&["exec", "-it", container_name, attach_command])
        .spawn()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Attaching to existing container".to_string(),
            file_path: None,
        })?;

    let status = child
        .wait()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Waiting for attach command".to_string(),
            file_path: None,
        })?;

    if !status.success() {
        return Err(DevshellError::DockerError(format!(
            "Attach command failed: container exited with status {:?}",
            status
        )));
    }

    Ok(())
}

pub fn prompt_container_action(container_name: &str) -> Result<String, DevshellError> {
    println!("Container '{}' already exists.", container_name);
    println!("Choose an action:");
    println!("[R]eplace container");
    println!("[A]ttach to container");
    println!("[Q]uit");

    print!("Your choice [R/a/q]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Reading user choice".to_string(),
            file_path: None,
        })?;

    match input.trim().to_lowercase().as_str() {
        "r" | "" => Ok("replace".to_string()),
        "a" => Ok("attach".to_string()),
        "q" => Ok("quit".to_string()),
        _ => {
            println!("Invalid choice. Please enter 'r', 'a', or 'q'.");
            prompt_container_action(container_name)
        }
    }
}
