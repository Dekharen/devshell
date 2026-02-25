use crate::config::schema::{Config, UserEntry};
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
        .args(["inspect", "--format={{.State.Status}}", container_name])
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
        .args(["inspect", "--format={{.State.Status}}", container_name])
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
        .args(["start", container_name])
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
    user: Option<&UserEntry>,
) -> Result<(), DevshellError> {
    // Start container in detached mode
    let mut run_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--userns=keep-id".to_string(),
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
        return Err(DevshellError::DockerError("Container failed to start within expected time".to_string()));
    }

    // Additional pause for container initialization
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Run chown if user is host_mirror
    if let Some(user_entry) = user {
        if let Some((host_name, proxy, home)) = user_entry.user.as_host_mirror() {
            run_chown(container_name, host_name, proxy, home)?;
        }
    }

    // Attach immediately
    let attach_cmd = config.attach_command.as_deref().unwrap_or("/bin/bash");

    let mut child = Command::new("docker")
        .args(["exec", "-it", container_name, attach_cmd])
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

fn run_chown(
    container_name: &str,
    host_name: &str,
    proxy: &str,
    home: &str,
) -> Result<(), DevshellError> {
    let marker_path = "/devshell/marker/.chowned";

    // Check if already chowned
    let check = Command::new("docker")
        .args(["exec", container_name, "test", "-f", marker_path])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Checking if chown already performed".to_string(),
            file_path: None,
        })?;

    if check.status.success() {
        eprintln!("DEBUG: Chown already performed, skipping");
        return Ok(());
    }

    // Create marker directory if it doesn't exist
    let mkdir_cmd = "mkdir -p /devshell/marker".to_string();
    Command::new("docker")
        .args(["exec", container_name, "sh", "-c", &mkdir_cmd])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Creating marker directory".to_string(),
            file_path: None,
        })?;

    // Count files first, then chown
    let count_cmd = format!("find {} -user {} 2>/dev/null | wc -l", home, proxy);
    let count_output = Command::new("docker")
        .args(["exec", container_name, "sh", "-c", &count_cmd])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Counting files to chown".to_string(),
            file_path: None,
        })?;

    let file_count = String::from_utf8_lossy(&count_output.stdout)
        .trim()
        .to_string();

    // Run chown command
    let chown_cmd = format!(
        "find {} -user {} -exec chown {}:{} {{}} + 2>/dev/null",
        home, proxy, host_name, host_name
    );

    Command::new("docker")
        .args(["exec", container_name, "sh", "-c", &chown_cmd])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Running chown".to_string(),
            file_path: None,
        })?;

    // Write marker file
    let marker_content =
        format!(
        "TIMESTAMP: {}\nCOMMAND: find {} -user {} -exec chown {}:{} {{}} +\nFILES_CHOWNED: {}\n",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        home, proxy, host_name, host_name, file_count
    );

    let write_marker_cmd = format!("cat > {} << 'EOF'\n{}EOF", marker_path, marker_content);

    Command::new("docker")
        .args(["exec", container_name, "sh", "-c", &write_marker_cmd])
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Writing marker file".to_string(),
            file_path: None,
        })?;

    eprintln!(
        "DEBUG: Chowned {} files from {} to {}",
        file_count, proxy, host_name
    );

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
        .args(["exec", "-it", container_name, attach_command])
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
