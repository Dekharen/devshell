use crate::error::{DevshellError, IoErrorContext};
use crate::fragments::resolve;
use std::fs;
use std::process::Command;

pub fn build_image(config_name: &str, stages: &[String]) -> Result<String, DevshellError> {
    let fragments = resolve::resolve_fragments(stages)?;
    let dockerfile_content = concatenate_fragments(&fragments);

    let temp_dir = std::env::temp_dir().join("devshell");
    fs::create_dir_all(&temp_dir).with_context_and_file(
        "Creating temporary build directory",
        &temp_dir.to_string_lossy(),
    )?;

    let dockerfile_path = temp_dir.join("Dockerfile");
    fs::write(&dockerfile_path, dockerfile_content).with_context_and_file(
        "Writing temporary Dockerfile",
        &dockerfile_path.to_string_lossy(),
    )?;

    let image_name = format!("devshell-{}", config_name);

    let output = Command::new("docker")
        .args(&["build", "-t", &image_name, "."])
        .current_dir(&temp_dir)
        .output()
        .map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: "Spawning docker build command".to_string(),
            file_path: None,
        })?;

    if !output.status.success() {
        return Err(DevshellError::DockerError(format!(
            "Docker build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(image_name)
}

fn concatenate_fragments(fragments: &[String]) -> String {
    let mut dockerfile = String::new();

    for fragment in fragments {
        dockerfile.push_str(fragment);
        dockerfile.push('\n');
    }

    dockerfile
}

pub fn cleanup_temp_files() -> Result<(), DevshellError> {
    let temp_dir = std::env::temp_dir().join("devshell");

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).with_context_and_file(
            "Cleaning up temporary build directory",
            &temp_dir.to_string_lossy(),
        )?;
    }

    Ok(())
}
