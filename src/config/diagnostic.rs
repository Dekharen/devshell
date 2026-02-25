use crate::config::schema::Config;
use crate::error::{DevshellError, Diagnosis};

pub struct ConfigDiagnostic;

impl ConfigDiagnostic {
    pub fn diagnose_all() -> Result<(), DevshellError> {
        println!("🔍 Analyzing configuration files...\n");

        let mut all_diagnoses = Vec::new();

        // Check embedded default config
        if let Err(e) = Self::diagnose_embedded_config() {
            match e {
                DevshellError::ConfigurationDiagnosis(mut diagnoses) => {
                    all_diagnoses.append(&mut diagnoses);
                }
                _ => return Err(e),
            }
        }

        // Check local configs
        if let Err(e) = Self::diagnose_local_configs() {
            match e {
                DevshellError::ConfigurationDiagnosis(mut diagnoses) => {
                    all_diagnoses.append(&mut diagnoses);
                }
                _ => return Err(e),
            }
        }

        // Check global configs
        if let Err(e) = Self::diagnose_global_configs() {
            match e {
                DevshellError::ConfigurationDiagnosis(mut diagnoses) => {
                    all_diagnoses.append(&mut diagnoses);
                }
                _ => return Err(e),
            }
        }

        if !all_diagnoses.is_empty() {
            Err(DevshellError::ConfigurationDiagnosis(all_diagnoses))
        } else {
            println!("✅ No configuration issues detected");
            Ok(())
        }
    }

    fn diagnose_embedded_config() -> Result<(), DevshellError> {
        let config_str = crate::fragments::embedded::get_default_config();
        Self::diagnose_config_string(config_str, Some("(embedded default)".to_string()))
    }

    fn diagnose_local_configs() -> Result<(), DevshellError> {
        let current_dir = std::env::current_dir()?;
        let mut configs_found = Vec::new();

        for entry in std::fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".devshell.toml") {
                        configs_found.push(path);
                    }
                }
            }
        }

        for config_path in &configs_found {
            let content = std::fs::read_to_string(config_path)?;
            let path_str = config_path.to_string_lossy().to_string();
            Self::diagnose_config_string(&content, Some(path_str))?;
        }

        Ok(())
    }

    fn diagnose_global_configs() -> Result<(), DevshellError> {
        let config_dir = crate::fs::get_config_dir();

        if !config_dir.exists() {
            return Ok(());
        }

        let mut configs_found = Vec::new();

        for entry in std::fs::read_dir(config_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".devshell.toml") {
                        configs_found.push(path);
                    }
                }
            }
        }

        for config_path in &configs_found {
            let content = std::fs::read_to_string(config_path)?;
            let path_str = config_path.to_string_lossy().to_string();
            Self::diagnose_config_string(&content, Some(path_str))?;
        }

        Ok(())
    }

    fn diagnose_config_string(
        config_str: &str,
        file_path: Option<String>,
    ) -> Result<(), DevshellError> {
        let mut diagnoses = Vec::new();

        // Parse as generic value first for structural analysis
        let raw_value: toml::Value =
            toml::from_str(config_str).map_err(DevshellError::InvalidToml)?;

        // Check for field misplacement issues
        Self::check_field_misplacement(&raw_value, &mut diagnoses, &file_path);

        // Try to parse as Config to validate schema
        if let Ok(config) = toml::from_str::<Config>(config_str) {
            // Check configuration consistency
            Self::check_configuration_consistency(&config, &mut diagnoses, &file_path);
        } else {
            // If we can't parse as Config, the InvalidToml error will be handled elsewhere
        }

        // Check for syntax warnings and best practices
        Self::check_syntax_best_practices(&raw_value, &mut diagnoses, &file_path);

        if !diagnoses.is_empty() {
            Err(DevshellError::ConfigurationDiagnosis(diagnoses))
        } else {
            Ok(())
        }
    }

    fn check_field_misplacement(
        raw_value: &toml::Value,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
        // Check if attach_command is inside volumes array (our main bug!)
        if let Some(volumes) = raw_value.get("volumes") {
            if let Some(volumes_array) = volumes.as_array() {
                for (i, volume) in volumes_array.iter().enumerate() {
                    if let Some(_attach_cmd) = volume.get("attach_command") {
                        diagnoses.push(Diagnosis::FieldMisplacement {
                            field: "attach_command".to_string(),
                            wrong_location: format!("inside [[volumes]] array item {}", i + 1),
                            suggestion:
                                "Move attach_command to top level before [[volumes]] section"
                                    .to_string(),
                            file_path: file_path.clone(),
                        });
                    }
                }
            }
        }

        // Check for other potential field misplacements
        if let Some(volumes) = raw_value.get("volumes") {
            if let Some(volumes_array) = volumes.as_array() {
                for (i, volume) in volumes_array.iter().enumerate() {
                    // Check for non-standard fields in volumes
                    if let Some(table) = volume.as_table() {
                        for key in table.keys() {
                            match key.as_str() {
                                "host" | "container" | "mode" => {
                                    // Valid volume fields
                                }
                                _ => {
                                    diagnoses.push(Diagnosis::SyntaxWarning {
                                        warning: format!("Unknown field `{}` in volume item {}", key, i + 1),
                                        suggestion: format!("Valid volume fields are: host, container, mode. Remove field `{}`.", key),
                                        file_path: file_path.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_configuration_consistency(
        config: &Config,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
        // Check if attach_command is present when keep-alive stage is used
        let has_keep_alive = config.stages.iter().any(|s| s.contains("keep-alive"));
        let has_attach_command = config.attach_command.is_some();

        if has_keep_alive && !has_attach_command {
            diagnoses.push(Diagnosis::MissingRequiredField {
                field: "attach_command".to_string(),
                reason: "Persistent containers (with keep-alive) need attach_command for interactive access".to_string(),
                suggestion: "Add: attach_command = \"/bin/bash\"".to_string(),
                file_path: file_path.clone(),
            });
        }

        // Check if attach_command is specified but no persistent container
        if !has_keep_alive && has_attach_command {
            diagnoses.push(Diagnosis::InconsistentConfiguration {
                issue: "attach_command specified but no persistent container configuration".to_string(),
                suggestion: "Add @post/keep-alive to stages array for persistent container, or remove attach_command".to_string(),
                file_path: file_path.clone(),
            });
        }

        // Check for empty volumes that might be useless
        for (i, volume) in config.volumes.iter().enumerate() {
            if volume.host.is_empty() {
                diagnoses.push(Diagnosis::SyntaxWarning {
                    warning: format!("Volume {} has empty host path", i + 1),
                    suggestion: "Provide a valid host path for the volume".to_string(),
                    file_path: file_path.clone(),
                });
            }

            if volume.container.is_empty() {
                diagnoses.push(Diagnosis::SyntaxWarning {
                    warning: format!("Volume {} has empty container path", i + 1),
                    suggestion: "Provide a valid container path for the volume".to_string(),
                    file_path: file_path.clone(),
                });
            }
        }

        // Check docker args for common issues
        for (i, arg) in config.docker_args.iter().enumerate() {
            if arg.is_empty() {
                diagnoses.push(Diagnosis::SyntaxWarning {
                    warning: format!("Docker argument {} is empty", i + 1),
                    suggestion: "Remove empty arguments or provide valid docker arguments"
                        .to_string(),
                    file_path: file_path.clone(),
                });
            }
        }

        // Check stages array
        if config.stages.is_empty() {
            diagnoses.push(Diagnosis::MissingRequiredField {
                field: "stages".to_string(),
                reason: "At least one stage is required to build a container".to_string(),
                suggestion: "Add: stages = [\"@base/debian\"]".to_string(),
                file_path: file_path.clone(),
            });
        }

        // Validate users
        Self::check_user_configuration(config, diagnoses, file_path);
    }

    fn check_user_configuration(
        config: &Config,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
        let user_count = config.users.len();
        let default_count = config.users.iter().filter(|u| u.default).count();

        if user_count == 0 {
            diagnoses.push(Diagnosis::MissingRequiredField {
                field: "user".to_string(),
                reason: "At least one user must be declared in the configuration".to_string(),
                suggestion: "Add a user to your config:\n\n\
[[user]]\nkind = \"container_local\"\nname = \"dev\"\ndefault = true\n\n\
Or for host user mirroring:\n\n\
[[user]]\nkind = \"host_mirror\"\nname = \"jenicola\"\nproxy = \"dev\"\nhome = \"/home/dev\"\ndefault = true".to_string(),
                file_path: file_path.clone(),
            });
        } else if default_count == 0 {
            diagnoses.push(Diagnosis::MissingRequiredField {
                field: "default user".to_string(),
                reason: "Exactly one user must be marked as default".to_string(),
                suggestion: "Add `default = true` to one user entry:\n\n\
[[user]]\nkind = \"container_local\"\nname = \"dev\"\ndefault = true"
                    .to_string(),
                file_path: file_path.clone(),
            });
        } else if default_count > 1 {
            diagnoses.push(Diagnosis::InconsistentConfiguration {
                issue: format!("{} users marked as default", default_count),
                suggestion: "Only one user can have `default = true`".to_string(),
                file_path: file_path.clone(),
            });
        }

        // Validate host_mirror users have required fields
        for (i, entry) in config.users.iter().enumerate() {
            if let crate::config::schema::User::HostMirror { name, proxy, home } = &entry.user {
                if name.is_empty() {
                    diagnoses.push(Diagnosis::SyntaxWarning {
                        warning: format!("User {} has empty name", i + 1),
                        suggestion: "Provide a valid name for host_mirror user".to_string(),
                        file_path: file_path.clone(),
                    });
                }
                if proxy.is_empty() {
                    diagnoses.push(Diagnosis::SyntaxWarning {
                        warning: format!("User {} has empty proxy", i + 1),
                        suggestion: "Provide a valid proxy name for host_mirror user".to_string(),
                        file_path: file_path.clone(),
                    });
                }
                if home.is_empty() {
                    diagnoses.push(Diagnosis::SyntaxWarning {
                        warning: format!("User {} has empty home", i + 1),
                        suggestion: "Provide a valid home path for host_mirror user".to_string(),
                        file_path: file_path.clone(),
                    });
                }
            }
        }
    }

    fn check_syntax_best_practices(
        raw_value: &toml::Value,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
        // Check for trailing commas in arrays (common TOML mistake)
        let config_str = raw_value.to_string();

        // Look for specific patterns that indicate common mistakes
        if config_str.contains("stages = ,") || config_str.contains("docker_args = ,") {
            diagnoses.push(Diagnosis::SyntaxWarning {
                warning: "Empty array detected".to_string(),
                suggestion: "Remove the comma or add valid items to the array".to_string(),
                file_path: file_path.clone(),
            });
        }

        // Check for missing quotes in string values
        if config_str.contains("attach_command = /bin/bash") && !config_str.contains("\"") {
            diagnoses.push(Diagnosis::SyntaxWarning {
                warning: "Unquoted string value detected".to_string(),
                suggestion: "Add quotes around string values: attach_command = \"/bin/bash\""
                    .to_string(),
                file_path: file_path.clone(),
            });
        }

        // Check for common argument formatting issues
        if config_str.contains("--memory=")
            && !config_str.contains("g")
            && !config_str.contains("m")
        {
            diagnoses.push(Diagnosis::SyntaxWarning {
                warning: "Memory value may need unit specification".to_string(),
                suggestion: "Specify memory with unit: --memory=4g or --memory=512m".to_string(),
                file_path: file_path.clone(),
            });
        }
    }
}
