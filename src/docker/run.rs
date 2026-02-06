use crate::config::schema::Config;
use crate::error::DevshellError;
use std::path::Path;
use std::process::Command;

pub fn run_container(config: &Config, image_name: &str) -> Result<(), DevshellError> {
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
