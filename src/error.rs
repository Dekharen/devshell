use std::fmt;

#[derive(Debug)]
pub enum DevshellError {
    ConfigNotFound(String),
    MultipleConfigs(Vec<String>),
    InvalidToml(toml::de::Error),
    FragmentNotFound(String),
    DockerError(String),
    DockerNotInstalled,
    DockerDaemonNotRunning,
    IoError(std::io::Error),
}

impl fmt::Display for DevshellError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DevshellError::ConfigNotFound(name) => write!(f, "Configuration not found: {}", name),
            DevshellError::MultipleConfigs(configs) => {
                write!(f, "Multiple configurations found: {:?}", configs)
            }
            DevshellError::InvalidToml(e) => write!(f, "Invalid TOML: {}", e),
            DevshellError::FragmentNotFound(fragment) => {
                write!(f, "Fragment not found: {}", fragment)
            }
            DevshellError::DockerError(msg) => write!(f, "Docker error: {}", msg),
            DevshellError::DockerNotInstalled => {
                write!(f, "Docker is not installed or not in PATH")
            }
            DevshellError::DockerDaemonNotRunning => write!(f, "Docker daemon is not running"),
            DevshellError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for DevshellError {}

impl From<toml::de::Error> for DevshellError {
    fn from(err: toml::de::Error) -> Self {
        DevshellError::InvalidToml(err)
    }
}

impl From<std::io::Error> for DevshellError {
    fn from(err: std::io::Error) -> Self {
        DevshellError::IoError(err)
    }
}
