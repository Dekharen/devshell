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
    IoErrorWithContext {
        error: std::io::Error,
        context: String,
        file_path: Option<String>,
    },
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
            DevshellError::IoErrorWithContext {
                error,
                context,
                file_path,
            } => {
                write!(f, "IO error: {}", context)?;
                if let Some(path) = file_path {
                    write!(f, " (file: {})", path)?;
                }
                write!(f, ": {}", error)
            }
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
        DevshellError::IoErrorWithContext {
            error: err,
            context: "Unknown operation".to_string(),
            file_path: None,
        }
    }
}

pub trait IoErrorContext<T> {
    fn with_context(self, context: &str) -> Result<T, DevshellError>;
    fn with_context_and_file(self, context: &str, file_path: &str) -> Result<T, DevshellError>;
}

impl<T> IoErrorContext<T> for Result<T, std::io::Error> {
    fn with_context(self, context: &str) -> Result<T, DevshellError> {
        self.map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: context.to_string(),
            file_path: None,
        })
    }

    fn with_context_and_file(self, context: &str, file_path: &str) -> Result<T, DevshellError> {
        self.map_err(|e| DevshellError::IoErrorWithContext {
            error: e,
            context: context.to_string(),
            file_path: Some(file_path.to_string()),
        })
    }
}
