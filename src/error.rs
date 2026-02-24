use std::fmt;

#[derive(Debug)]
pub enum DevshellError {
    ConfigNotFound(String),
    MultipleConfigs(Vec<String>),
    InvalidToml(toml::de::Error),
    FragmentNotFound(String),
    DockerError(String),
    UserNotFound {
        requested: String,
        suggestion: String,
    },
    NoUserDeclared,
    NoDefaultUser,
    MultipleDefaultUsers,
    IoErrorWithContext {
        error: std::io::Error,
        context: String,
        file_path: Option<String>,
    },
    ConfigurationDiagnosis(Vec<Diagnosis>),
}

#[derive(Debug)]
pub enum Diagnosis {
    FieldMisplacement {
        field: String,
        wrong_location: String,
        suggestion: String,
        file_path: Option<String>,
    },
    MissingRequiredField {
        field: String,
        reason: String,
        suggestion: String,
        file_path: Option<String>,
    },
    InconsistentConfiguration {
        issue: String,
        suggestion: String,
        file_path: Option<String>,
    },
    SyntaxWarning {
        warning: String,
        suggestion: String,
        file_path: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Diagnosis {
    pub fn severity(&self) -> Severity {
        match self {
            Diagnosis::FieldMisplacement { .. } => Severity::Error,
            Diagnosis::MissingRequiredField { .. } => Severity::Warning,
            Diagnosis::InconsistentConfiguration { .. } => Severity::Warning,
            Diagnosis::SyntaxWarning { .. } => Severity::Info,
        }
    }

    pub fn file_path(&self) -> Option<&String> {
        match self {
            Diagnosis::FieldMisplacement { file_path, .. } => file_path.as_ref(),
            Diagnosis::MissingRequiredField { file_path, .. } => file_path.as_ref(),
            Diagnosis::InconsistentConfiguration { file_path, .. } => file_path.as_ref(),
            Diagnosis::SyntaxWarning { file_path, .. } => file_path.as_ref(),
        }
    }
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
            DevshellError::UserNotFound {
                requested,
                suggestion,
            } => {
                write!(
                    f,
                    "User '{}' not found in configuration.\n\n{}",
                    requested, suggestion
                )
            }
            DevshellError::NoUserDeclared => {
                write!(f, "No users declared in configuration.\n\nAdd at least one user to your devshell config:\n\n\
[[user]]\nkind = \"container_local\"\nname = \"dev\"\ndefault = true\n\n\
Or for host user mirroring:\n\n\
[[user]]\nkind = \"host_mirror\"\nname = \"jenicola\"\nproxy = \"dev\"\nhome = \"/home/dev\"\ndefault = true")
            }
            DevshellError::NoDefaultUser => {
                write!(
                    f,
                    "No default user marked in configuration.\n\n\
Exactly one user must have `default = true` in your devshell config:\n\n\
[[user]]\nkind = \"container_local\"\nname = \"dev\"\ndefault = true"
                )
            }
            DevshellError::MultipleDefaultUsers => {
                write!(
                    f,
                    "Multiple users marked as default.\n\n\
Only one user can have `default = true` in your devshell config."
                )
            }
            // DevshellError::DockerNotInstalled => {
            // write!(f, "Docker is not installed or not in PATH")
            // }
            // DevshellError::DockerDaemonNotRunning => write!(f, "Docker daemon is not running"),
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
            DevshellError::ConfigurationDiagnosis(diagnoses) => {
                write!(f, "Configuration issues detected")?;
                for (i, diagnosis) in diagnoses.iter().enumerate() {
                    write!(
                        f,
                        "\n\n{}: {}",
                        i + 1,
                        crate::util::format_diagnosis(diagnosis)
                    )?;
                }
                Ok(())
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
