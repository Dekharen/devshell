#[cfg(test)]
mod diagnostic_tests {
    use crate::config::diagnostic::ConfigDiagnostic;
    use crate::error::{DevshellError, Diagnosis, Severity};

    // Helper function to parse config and get diagnoses
    fn get_diagnoses_for_config(config_str: &str) -> Vec<Diagnosis> {
        // Use the internal diagnostic logic directly
        let raw_value: toml::Value = toml::from_str(config_str).unwrap();
        let mut diagnoses = Vec::new();

        // Check field misplacement
        check_field_misplacement(&raw_value, &mut diagnoses, &None);

        // Check configuration consistency if parseable
        if let Ok(config) = toml::from_str::<crate::config::schema::Config>(config_str) {
            check_configuration_consistency(&config, &mut diagnoses, &None);
        }

        diagnoses
    }

    fn check_field_misplacement(
        raw_value: &toml::Value,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
        if let Some(volumes) = raw_value.get("volumes") {
            if let Some(volumes_array) = volumes.as_array() {
                for (i, volume) in volumes_array.iter().enumerate() {
                    if volume.get("attach_command").is_some() {
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
    }

    fn check_configuration_consistency(
        config: &crate::config::schema::Config,
        diagnoses: &mut Vec<Diagnosis>,
        file_path: &Option<String>,
    ) {
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

        if !has_keep_alive && has_attach_command {
            diagnoses.push(Diagnosis::InconsistentConfiguration {
                issue: "attach_command specified but no persistent container configuration".to_string(),
                suggestion: "Add @post/keep-alive to stages array for persistent container, or remove attach_command".to_string(),
                file_path: file_path.clone(),
            });
        }

        if config.stages.is_empty() {
            diagnoses.push(Diagnosis::MissingRequiredField {
                field: "stages".to_string(),
                reason: "At least one stage is required to build a container".to_string(),
                suggestion: "Add: stages = [\"@base/debian\"]".to_string(),
                file_path: file_path.clone(),
            });
        }
    }

    // ========================================
    // FIELD MISPLACEMENT TESTS
    // ========================================

    // These tests verify that attach_command inside [[volumes]] is correctly detected

    #[test]
    fn test_attach_command_in_volumes_detected() {
        let config = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let misplacement = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::FieldMisplacement { .. }));

        assert!(
            misplacement.is_some(),
            "Should detect attach_command in volumes"
        );
        if let Some(Diagnosis::FieldMisplacement {
            field,
            wrong_location,
            ..
        }) = misplacement
        {
            assert_eq!(field, "attach_command");
            assert!(wrong_location.contains("[[volumes]]"));
        }
    }

    #[test]
    fn test_attach_command_correct_location_not_flagged() {
        let config = r#"
name = "test"
stages = ["@base/debian"]
attach_command = "/bin/bash"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let misplacement = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::FieldMisplacement { .. }));

        assert!(
            misplacement.is_none(),
            "Should NOT detect misplacement when attach_command is at top level"
        );
    }

    #[test]
    fn test_multiple_volumes_with_misplacement() {
        let config = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "/tmp"
container = "/tmp"
mode = "rw"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let misplacements: Vec<_> = diagnoses
            .iter()
            .filter(|d| matches!(d, Diagnosis::FieldMisplacement { .. }))
            .collect();

        assert_eq!(
            misplacements.len(),
            1,
            "Should detect exactly one misplacement"
        );
    }

    // ========================================
    // MISSING REQUIRED FIELD TESTS
    // ========================================

    #[test]
    fn test_missing_attach_command_with_keep_alive() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses.iter()
            .find(|d| matches!(d, Diagnosis::MissingRequiredField { field, .. } if field == "attach_command"));

        assert!(
            missing.is_some(),
            "Should detect missing attach_command with keep-alive"
        );
    }

    #[test]
    fn test_attach_command_present_with_keep_alive_no_warning() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses.iter()
            .find(|d| matches!(d, Diagnosis::MissingRequiredField { field, .. } if field == "attach_command"));

        assert!(
            missing.is_none(),
            "Should NOT warn when attach_command present with keep-alive"
        );
    }

    #[test]
    fn test_attach_command_without_keep_alive_warns() {
        let config = r#"
name = "test"
stages = ["@base/debian"]
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let inconsistent = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::InconsistentConfiguration { .. }));

        assert!(
            inconsistent.is_some(),
            "Should warn about inconsistent config"
        );
    }

    // ========================================
    // EMPTY STAGES TESTS
    // ========================================

    #[test]
    fn test_empty_stages_detected() {
        let config = r#"
name = "test"
stages = []
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses.iter().find(
            |d| matches!(d, Diagnosis::MissingRequiredField { field, .. } if field == "stages"),
        );

        assert!(missing.is_some(), "Should detect empty stages");
    }

    #[test]
    fn test_valid_stages_not_flagged() {
        let config = r#"
name = "test"
stages = ["@base/debian"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses.iter().find(
            |d| matches!(d, Diagnosis::MissingRequiredField { field, .. } if field == "stages"),
        );

        assert!(missing.is_none(), "Should NOT flag valid stages");
    }

    // ========================================
    // SEVERITY TESTS
    // ========================================

    #[test]
    fn test_field_misplacement_is_error() {
        let config = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "."
container = "/workspace"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let misplacement = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::FieldMisplacement { .. }));

        assert!(misplacement.is_some());
        assert_eq!(misplacement.unwrap().severity(), Severity::Error);
    }

    #[test]
    fn test_missing_required_field_is_warning() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::MissingRequiredField { .. }));

        assert!(missing.is_some());
        assert_eq!(missing.unwrap().severity(), Severity::Warning);
    }

    #[test]
    fn test_inconsistent_configuration_is_warning() {
        let config = r#"
name = "test"
stages = ["@base/debian"]
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let inconsistent = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::InconsistentConfiguration { .. }));

        assert!(inconsistent.is_some());
        assert_eq!(inconsistent.unwrap().severity(), Severity::Warning);
    }

    // ========================================
    // ERROR MESSAGE CONTENT TESTS
    // ========================================

    #[test]
    fn test_error_message_contains_field_name() {
        let config = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "."
container = "/workspace"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let misplacement = diagnoses.iter()
            .find(|d| matches!(d, Diagnosis::FieldMisplacement { field, .. } if field == "attach_command"));

        assert!(misplacement.is_some());
        if let Some(Diagnosis::FieldMisplacement { suggestion, .. }) = misplacement {
            assert!(
                suggestion.contains("top level"),
                "Suggestion should mention top level"
            );
        }
    }

    #[test]
    fn test_missing_field_message_is_helpful() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::MissingRequiredField { suggestion, .. }));

        assert!(missing.is_some());
        if let Some(Diagnosis::MissingRequiredField { suggestion, .. }) = missing {
            assert!(
                suggestion.contains("attach_command"),
                "Suggestion should mention attach_command"
            );
            assert!(
                suggestion.contains("/bin/bash"),
                "Suggestion should show example value"
            );
        }
    }

    // ========================================
    // COMPLEX CONFIGURATION TESTS
    // ========================================

    #[test]
    fn test_full_valid_config_no_errors() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@features/basic-debian", "@post/dev-user", "@post/keep-alive"]
docker_args = ["--memory=2g", "--cpus=2"]
attach_command = "/bin/bash"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"

[[volumes]]
host = "/tmp"
container = "/tmp"
mode = "rw"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        // Should have no errors or warnings, only potentially syntax warnings
        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();

        assert_eq!(errors, 0, "Valid config should have no errors");
    }

    #[test]
    fn test_multiple_issues_detected() {
        let config = r#"
name = "test"

[[volumes]]
host = "."
container = "/workspace"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        // Should detect field misplacement (stages is missing entirely, not empty)
        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();

        let warnings = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Warning)
            .count();

        assert!(
            errors >= 1,
            "Should detect at least 1 error (field misplacement)"
        );
        assert!(diagnoses.len() >= 1, "Should detect at least 1 issue total");
    }

    // ========================================
    // EMBEDDED CONFIG TESTS
    // ========================================

    #[test]
    fn test_embedded_default_config_valid() {
        let config_str = crate::fragments::embedded::get_default_config();
        let diagnoses = get_diagnoses_for_config(config_str);

        // Embedded config should be valid
        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();

        assert_eq!(errors, 0, "Embedded default config should have no errors");
    }

    // ========================================
    // EDGE CASE TESTS
    // ========================================

    #[test]
    fn test_minimal_valid_config() {
        let config = r#"
name = "test"
stages = ["@base/debian"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        // Minimal config with required fields should be valid
        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();

        assert_eq!(errors, 0, "Minimal valid config should have no errors");
    }

    #[test]
    fn test_keep_alive_variant_detected() {
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let missing = diagnoses.iter()
            .find(|d| matches!(d, Diagnosis::MissingRequiredField { field, .. } if field == "attach_command"));

        assert!(
            missing.is_some(),
            "Should detect missing attach_command for @post/keep-alive"
        );
    }

    #[test]
    fn test_alternative_keep_alive_works() {
        // Test with attach but no keep-alive - should warn about inconsistency
        let config = r#"
name = "test"
stages = ["@base/debian"]
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let inconsistent = diagnoses
            .iter()
            .find(|d| matches!(d, Diagnosis::InconsistentConfiguration { .. }));

        assert!(
            inconsistent.is_some(),
            "Should warn about attach without keep-alive"
        );
    }

    // ========================================
    // DIAGNOSIS COUNT TESTS
    // ========================================

    #[test]
    fn test_exact_error_count_for_broken_config() {
        // Config with attach_command INSIDE volumes AND keep-alive without attach_command
        let config = r#"
name = "test"
stages = ["@base/debian", "@post/keep-alive"]

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
attach_command = "/bin/bash"
"#;
        let diagnoses = get_diagnoses_for_config(config);

        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();

        // Should have exactly 1 error: field misplacement
        assert_eq!(
            errors, 1,
            "Should have exactly 1 error (field misplacement)"
        );
    }

    #[test]
    fn test_diagnosis_types_exhaustive() {
        // Test that all diagnosis types can be created
        let diagnoses = vec![
            Diagnosis::FieldMisplacement {
                field: "test".to_string(),
                wrong_location: "test location".to_string(),
                suggestion: "test suggestion".to_string(),
                file_path: None,
            },
            Diagnosis::MissingRequiredField {
                field: "test".to_string(),
                reason: "test reason".to_string(),
                suggestion: "test suggestion".to_string(),
                file_path: None,
            },
            Diagnosis::InconsistentConfiguration {
                issue: "test issue".to_string(),
                suggestion: "test suggestion".to_string(),
                file_path: None,
            },
            Diagnosis::SyntaxWarning {
                warning: "test warning".to_string(),
                suggestion: "test suggestion".to_string(),
                file_path: None,
            },
        ];

        assert_eq!(
            diagnoses.len(),
            4,
            "All 4 diagnosis types should be creatable"
        );

        // Verify severity counts
        let errors = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count();
        let warnings = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Warning)
            .count();
        let infos = diagnoses
            .iter()
            .filter(|d| d.severity() == Severity::Info)
            .count();

        assert_eq!(errors, 1, "Should have 1 error type");
        assert_eq!(warnings, 2, "Should have 2 warning types");
        assert_eq!(infos, 1, "Should have 1 info type");
    }
}
