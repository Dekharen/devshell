use crate::config::schema::Config;
use crate::error::DevshellError;
use std::process::Command;

pub fn run_container(config: &Config, image_name: &str) -> Result<(), DevshellError> {
    let mut args = vec!["run".to_string(), "--rm".to_string()];

    args.extend(config.docker_args.clone());

    for volume in &config.volumes {
        args.push(format!(
            "{}:{}:{}",
            volume.host, volume.container, volume.mode
        ));
    }

    args.push("-it".to_string());
    args.push(image_name.to_string());

    let output = Command::new("docker").args(&args).output()?;

    if !output.status.success() {
        return Err(DevshellError::DockerError(format!(
            "Docker run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}
