#[cfg(test)]
mod config_parsing_tests {
    use crate::config::schema::{Config, Volume};
    use crate::fragments::embedded;

    // ========================================
    // BASIC PARSING VALIDATION TESTS
    // ========================================

    // These tests verify that basic TOML parsing works correctly for all fields
    // Should catch fundamental serialization/deserialization issues

    #[test]
    fn test_parse_minimal_config() {
        // Test absolute minimum required fields
        let config_content = r#"
name = "minimal"
stages = ["@base/debian"]
"#;
        let config: Config = toml::from_str(config_content).expect("Should parse minimal config");

        assert_eq!(config.name, "minimal");
        assert_eq!(config.stages, vec!["@base/debian"]);
        assert!(config.docker_args.is_empty());
        assert!(config.volumes.is_empty());
        assert!(config.attach_command.is_none());
    }

    #[test]
    fn test_parse_full_config() {
        // Test all fields populated
        let config_content = r#"
name = "full-config"
stages = ["@base/debian", "@features/basic-debian"]
docker_args = ["--memory=2g", "--cpus=4"]
attach_command = "/bin/zsh"

[[volumes]]
host = "/tmp"
container = "/host-tmp"
mode = "rw"

[[volumes]]
host = "."
container = "/workspace"
mode = "ro"
"#;
        let config: Config = toml::from_str(config_content).expect("Should parse full config");

        assert_eq!(config.name, "full-config");
        assert_eq!(config.stages.len(), 2);
        assert_eq!(config.docker_args.len(), 2);
        assert_eq!(config.volumes.len(), 2);
        assert_eq!(config.attach_command, Some("/bin/zsh".to_string()));
    }

    // ========================================
    // FIELD ORDERING TESTS
    // ========================================

    // These tests ensure field order doesn't affect parsing (critical TOML edge case)
    // Should catch the attach_command/volumes association bug we just fixed

    #[test]
    fn test_field_order_attach_command_first() {
        let config_content = r#"
attach_command = "/bin/bash"
name = "test"
stages = ["@base/debian"]
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse with attach_command first");
        assert_eq!(config.attach_command, Some("/bin/bash".to_string()));
    }

    #[test]
    fn test_field_order_volumes_before_attach_command() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]
attach_command = "/bin/bash"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
"#;
        let config: Config = toml::from_str(config_content)
            .expect("Should parse with attach_command before volumes");
        assert_eq!(config.attach_command, Some("/bin/bash".to_string()));
        assert_eq!(config.volumes.len(), 1);
    }

    #[test]
    fn test_field_order_all_permutations() {
        // Test all possible field orderings to catch any ordering bugs
        let fields = vec![
            "name = \"test\"",
            "stages = [\"@base/debian\"]",
            "attach_command = \"/bin/bash\"",
            "docker_args = [\"--memory=1g\"]",
        ];

        // Test a few key permutations (not all 24 to keep test reasonable)
        let permutations = vec![
            vec![0, 1, 2, 3], // Original order
            vec![3, 2, 1, 0], // Reverse order
            vec![2, 0, 1, 3], // attach_command first
            vec![0, 3, 1, 2], // docker_args early
        ];

        for perm in permutations {
            let config_content = perm
                .iter()
                .map(|&i| fields[i])
                .collect::<Vec<_>>()
                .join("\n");

            let config: Config = toml::from_str(&config_content)
                .unwrap_or_else(|e| panic!("Failed to parse permutation {:?}: {}", perm, e));

            assert_eq!(config.name, "test");
            assert_eq!(config.attach_command, Some("/bin/bash".to_string()));
        }
    }

    // ========================================
    // VOLUME ARRAY TESTS
    // ========================================

    // These tests specifically target the [[volumes]] array parsing
    // Should catch edge cases in array of tables parsing

    #[test]
    fn test_single_volume() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "/tmp"
container = "/host-tmp"
mode = "rw"
"#;
        let config: Config = toml::from_str(config_content).expect("Should parse single volume");
        assert_eq!(config.volumes.len(), 1);
        assert_eq!(config.volumes[0].host, "/tmp");
        assert_eq!(config.volumes[0].container, "/host-tmp");
        assert_eq!(config.volumes[0].mode, "rw");
    }

    #[test]
    fn test_multiple_volumes() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[volumes]]
host = "/tmp"
container = "/host-tmp"
mode = "rw"

[[volumes]]
host = "/var/log"
container = "/logs"
mode = "ro"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
"#;
        let config: Config = toml::from_str(config_content).expect("Should parse multiple volumes");
        assert_eq!(config.volumes.len(), 3);

        let tmp_path = &"/tmp".to_string();
        let tmp_container = &"/host-tmp".to_string();
        let log_path = &"/var/log".to_string();
        let log_container = &"/logs".to_string();
        let dot_path = &".".to_string();
        let workspace_container = &"/workspace".to_string();

        let volume_paths: Vec<_> = config
            .volumes
            .iter()
            .map(|v| (&v.host, &v.container))
            .collect();

        assert!(volume_paths.contains(&(tmp_path, tmp_container)));
        assert!(volume_paths.contains(&(log_path, log_container)));
        assert!(volume_paths.contains(&(dot_path, workspace_container)));
    }

    // ========================================
    // ATTACH_COMMAND EDGE CASES
    // ========================================

    // These tests specifically target attach_command parsing edge cases
    // Should catch various string parsing issues

    #[test]
    fn test_attach_command_variations() {
        let test_cases = vec![
            ("/bin/bash", "/bin/bash"),
            ("/bin/sh", "/bin/sh"),
            ("/usr/bin/zsh", "/usr/bin/zsh"),
            ("bash", "bash"),
            ("", ""), // Empty string
            ("/bin/bash -c 'echo hello'", "/bin/bash -c 'echo hello'"),
        ];

        for (input, expected) in test_cases {
            let config_content = if input.is_empty() {
                r#"
name = "test"
stages = ["@base/debian"]
attach_command = ""
"#
                .to_string()
            } else {
                format!(
                    r#"
name = "test"
stages = ["@base/debian"]
attach_command = "{}"
"#,
                    input
                )
            };

            let config: Config = toml::from_str(&config_content)
                .unwrap_or_else(|e| panic!("Failed to parse attach_command '{}': {}", input, e));

            assert_eq!(config.attach_command, Some(expected.to_string()));
        }
    }

    // ========================================
    // SERIALIZATION ROUNDTRIP TESTS
    // ========================================

    // These tests verify that parsing -> serialization -> parsing works
    // Should catch any asymmetry in serialization/deserialization

    #[test]
    fn test_roundtrip_full_config() {
        let original = Config {
            name: "roundtrip-test".to_string(),
            stages: vec![
                "@base/debian".to_string(),
                "@features/basic-debian".to_string(),
            ],
            docker_args: vec!["--memory=2g".to_string(), "--cpus=4".to_string()],
            volumes: vec![Volume {
                host: "/tmp".to_string(),
                container: "/host-tmp".to_string(),
                mode: "rw".to_string(),
            }],
            attach_command: Some("/bin/zsh".to_string()),
            users: vec![],
        };

        let serialized = toml::to_string_pretty(&original).expect("Should serialize");

        let deserialized: Config =
            toml::from_str(&serialized).expect("Should deserialize serialized config");

        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.stages, deserialized.stages);
        assert_eq!(original.docker_args, deserialized.docker_args);
        assert_eq!(original.volumes.len(), deserialized.volumes.len());
        assert_eq!(original.attach_command, deserialized.attach_command);
    }

    // ========================================
    // EMBEDDED CONFIG TESTS
    // ========================================

    #[test]
    fn test_embedded_default_config_consistency() {
        // Ensure the embedded config we ship actually parses correctly
        let config_str = embedded::get_default_config();
        let config: Config =
            toml::from_str(config_str).expect("Embedded default config should be valid TOML");

        assert!(!config.name.is_empty());
        assert!(!config.stages.is_empty());
        assert!(config.attach_command.is_some());
        assert!(!config.attach_command.unwrap().is_empty());
    }

    // ========================================
    // FUZZING TESTS
    // ========================================

    // These tests use semi-randomized inputs to find edge cases
    // Should catch unexpected combinations that break parsing

    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    fn generate_random_config(seed: u64) -> String {
        let mut rng = StdRng::seed_from_u64(seed);

        let names = vec!["test", "dev", "app", "project", "workspace"];
        let shells = vec!["/bin/bash", "/bin/sh", "/bin/zsh", "/usr/bin/fish"];
        let stages = vec![
            "@base/debian",
            "@base/ubuntu",
            "@features/basic-debian",
            "@post/dev-user",
        ];

        let name = names[rng.gen_range(0..names.len())];
        let attach_cmd = shells[rng.gen_range(0..shells.len())];

        let mut config = format!(
            r#"
name = "{}"
stages = ["{}"]"#,
            name,
            stages[rng.gen_range(0..stages.len())]
        );

        // Randomly add attach_command
        if rng.gen_bool(0.7) {
            config.push_str(&format!("\nattach_command = \"{}\"", attach_cmd));
        }

        // Randomly add docker_args
        if rng.gen_bool(0.5) {
            let memory = rng.gen_range(1..8);
            config.push_str(&format!("\ndocker_args = [\"--memory={}g\"]", memory));
        }

        // Randomly add volumes
        if rng.gen_bool(0.6) {
            config.push_str(
                r#"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw""#,
            );
        }

        // Random field order
        if rng.gen_bool(0.3) {
            // Move attach_command to front if it exists
            if config.contains("attach_command") {
                let attach_line: String = config
                    .lines()
                    .find(|line| line.contains("attach_command"))
                    .unwrap_or("")
                    .to_string();

                if !attach_line.is_empty() {
                    config = config.replace(&attach_line, "");
                    config = format!("{}\n{}", attach_line.trim(), config);
                }
            }
        }

        config
    }

    #[test]
    fn test_fuzzed_configs() {
        // Test 20 random configurations with different seeds
        for seed in 0..20 {
            let config_str = generate_random_config(seed);
            let result: Result<Config, _> = toml::from_str(&config_str);

            match result {
                Ok(config) => {
                    // Validate the parsed config is consistent
                    assert!(
                        !config.name.is_empty(),
                        "Seed {}: name should not be empty",
                        seed
                    );
                    assert!(
                        !config.stages.is_empty(),
                        "Seed {}: stages should not be empty",
                        seed
                    );

                    // If attach_command exists, it should not be empty
                    if let Some(ref cmd) = config.attach_command {
                        assert!(
                            !cmd.is_empty(),
                            "Seed {}: attach_command should not be empty",
                            seed
                        );
                    }

                    // Validate volumes if present
                    for (i, volume) in config.volumes.iter().enumerate() {
                        assert!(
                            !volume.host.is_empty(),
                            "Seed {}: volume {} host should not be empty",
                            seed,
                            i
                        );
                        assert!(
                            !volume.container.is_empty(),
                            "Seed {}: volume {} container should not be empty",
                            seed,
                            i
                        );
                        assert!(
                            !volume.mode.is_empty(),
                            "Seed {}: volume {} mode should not be empty",
                            seed,
                            i
                        );
                    }
                }
                Err(e) => {
                    // Parse errors should be reasonable TOML errors, not panics
                    let error_msg = e.to_string().to_lowercase();
                    assert!(
                        error_msg.contains("toml")
                            || error_msg.contains("parse")
                            || error_msg.contains("expected"),
                        "Seed {}: Parse error should be TOML-related: {}",
                        seed,
                        e
                    );
                }
            }
        }
    }

    // ========================================
    // INTEGRATION TESTS
    // ========================================

    #[test]
    fn test_real_world_config_scenarios() {
        // Test configs that resemble real-world usage

        let scenarios = vec![
            // Basic web development
            r#"
name = "web-dev"
stages = ["@base/debian", "@features/basic-debian", "@post/dev-user", "@post/keep-alive"]
docker_args = ["--memory=2g", "--cpus=2"]
attach_command = "/bin/bash"

[[volumes]]
host = "."
container = "/app"
mode = "rw"
"#,
            // Minimal container
            r#"
name = "minimal"
stages = ["@base/debian"]
attach_command = "/bin/sh"
"#,
        ];

        for (i, config_str) in scenarios.iter().enumerate() {
            let config: Config = toml::from_str(config_str)
                .unwrap_or_else(|e| panic!("Scenario {} failed: {}", i, e));

            assert!(
                !config.name.is_empty(),
                "Scenario {}: name should not be empty",
                i
            );
            assert!(
                !config.stages.is_empty(),
                "Scenario {}: stages should not be empty",
                i
            );

            // Validate attach_command is present for persistent containers
            if config.stages.iter().any(|s| s.contains("keep-alive")) {
                assert!(
                    config.attach_command.is_some(),
                    "Scenario {} with keep-alive should have attach_command",
                    i
                );
            }
        }
    }

    // ========================================
    // USER SCHEMA PARSING TESTS
    // ========================================

    // These tests verify that the user configuration schema parses correctly
    // Tests for all 4 user kinds: host_mirror, container_local, named_existing, root

    #[test]
    fn test_parse_container_local_user() {
        // Test absolute minimum required fields
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "container_local"
name = "dev"
default = true
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse config with container_local user");

        assert_eq!(config.users.len(), 1);
        let user = &config.users[0];
        assert!(
            matches!(&user.user, crate::config::schema::User::ContainerLocal { name, .. } if name == "dev")
        );
        assert!(user.default);
    }

    #[test]
    fn test_parse_container_local_user_with_optional_fields() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "container_local"
name = "dev"
home = "/home/dev"
shell = "/bin/zsh"
default = true
"#;
        let config: Config = toml::from_str(config_content)
            .expect("Should parse config with all container_local fields");

        let user = &config.users[0];
        if let crate::config::schema::User::ContainerLocal { name, home, shell } = &user.user {
            assert_eq!(name, "dev");
            assert_eq!(home, &Some("/home/dev".to_string()));
            assert_eq!(shell, &Some("/bin/zsh".to_string()));
        } else {
            panic!("Expected ContainerLocal user");
        }
    }

    #[test]
    fn test_parse_host_mirror_user() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "host_mirror"
name = "jenicola"
proxy = "dev"
home = "/home/dev"
default = true
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse config with host_mirror user");

        assert_eq!(config.users.len(), 1);
        let user = &config.users[0];
        assert!(
            matches!(&user.user, crate::config::schema::User::HostMirror { name, proxy, home } 
            if name == "jenicola" && proxy == "dev" && home == "/home/dev")
        );
        assert!(user.default);
    }

    #[test]
    fn test_parse_named_existing_user() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "named_existing"
name = "node"
default = true
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse config with named_existing user");

        assert_eq!(config.users.len(), 1);
        let user = &config.users[0];
        assert!(
            matches!(&user.user, crate::config::schema::User::NamedExisting { name, .. } if name == "node")
        );
        assert!(user.default);
    }

    #[test]
    fn test_parse_named_existing_user_with_home() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "named_existing"
name = "www-data"
home = "/var/www"
default = true
"#;
        let config: Config = toml::from_str(config_content)
            .expect("Should parse config with named_existing user and home");

        let user = &config.users[0];
        if let crate::config::schema::User::NamedExisting { name, home } = &user.user {
            assert_eq!(name, "www-data");
            assert_eq!(home, &Some("/var/www".to_string()));
        } else {
            panic!("Expected NamedExisting user");
        }
    }

    #[test]
    fn test_parse_root_user() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "root"
default = true
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse config with root user");

        assert_eq!(config.users.len(), 1);
        let user = &config.users[0];
        assert!(matches!(&user.user, crate::config::schema::User::Root {}));
        assert!(user.default);
    }

    #[test]
    fn test_parse_multiple_users() {
        let config_content = r#"
name = "test"
stages = ["@base/debian"]

[[users]]
kind = "container_local"
name = "dev"
default = true

[[users]]
kind = "host_mirror"
name = "jenicola"
proxy = "dev"
home = "/home/dev"

[[users]]
kind = "root"
"#;
        let config: Config =
            toml::from_str(config_content).expect("Should parse config with multiple users");

        assert_eq!(config.users.len(), 3);

        // First user is default
        assert!(config.users[0].default);
        assert!(
            matches!(&config.users[0].user, crate::config::schema::User::ContainerLocal { name, .. } if name == "dev")
        );

        // Second user is not default
        assert!(!config.users[1].default);
        assert!(
            matches!(&config.users[1].user, crate::config::schema::User::HostMirror { name, .. } if name == "jenicola")
        );

        // Third user is not default
        assert!(!config.users[2].default);
        assert!(matches!(
            &config.users[2].user,
            crate::config::schema::User::Root {}
        ));
    }

    // ========================================
    // USER VALIDATION TESTS
    // ========================================

    // These tests verify that user validation works correctly

    #[test]
    fn test_validate_users_no_users() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![],
        };

        let result = config.validate_users();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::DevshellError::NoUserDeclared
        ));
    }

    #[test]
    fn test_validate_users_no_default() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![crate::config::schema::UserEntry {
                user: crate::config::schema::User::ContainerLocal {
                    name: "dev".to_string(),
                    home: None,
                    shell: None,
                },
                default: false,
            }],
        };

        let result = config.validate_users();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::DevshellError::NoDefaultUser
        ));
    }

    #[test]
    fn test_find_user_explicit() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::ContainerLocal {
                        name: "dev".to_string(),
                        home: None,
                        shell: None,
                    },
                    default: true,
                },
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::HostMirror {
                        name: "jenicola".to_string(),
                        proxy: "dev".to_string(),
                        home: "/home/dev".to_string(),
                    },
                    default: false,
                },
            ],
        };

        let result = config.find_user_or_default(Some("jenicola"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "jenicola");
    }

    #[test]
    fn test_find_user_not_found() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![crate::config::schema::UserEntry {
                user: crate::config::schema::User::ContainerLocal {
                    name: "dev".to_string(),
                    home: None,
                    shell: None,
                },
                default: true,
            }],
        };

        let result = config.find_user_or_default(Some("nonexistent"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::DevshellError::UserNotFound { .. }
        ));
    }

    #[test]
    fn test_find_user_default() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::ContainerLocal {
                        name: "dev".to_string(),
                        home: None,
                        shell: None,
                    },
                    default: false,
                },
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::HostMirror {
                        name: "jenicola".to_string(),
                        proxy: "dev".to_string(),
                        home: "/home/dev".to_string(),
                    },
                    default: true,
                },
            ],
        };

        let result = config.find_user_or_default(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "jenicola");
    }

    #[test]
    fn test_get_host_mirror_user() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::ContainerLocal {
                        name: "dev".to_string(),
                        home: None,
                        shell: None,
                    },
                    default: true,
                },
                crate::config::schema::UserEntry {
                    user: crate::config::schema::User::HostMirror {
                        name: "jenicola".to_string(),
                        proxy: "dev".to_string(),
                        home: "/home/dev".to_string(),
                    },
                    default: false,
                },
            ],
        };

        let result = config.get_host_mirror_user();
        assert!(result.is_some());
        let (name, proxy, home) = result.unwrap();
        assert_eq!(name, "jenicola");
        assert_eq!(proxy, "dev");
        assert_eq!(home, "/home/dev");
    }

    #[test]
    fn test_get_host_mirror_user_none() {
        let config = Config {
            name: "test".to_string(),
            stages: vec!["@base/debian".to_string()],
            docker_args: vec![],
            volumes: vec![],
            attach_command: None,
            users: vec![crate::config::schema::UserEntry {
                user: crate::config::schema::User::ContainerLocal {
                    name: "dev".to_string(),
                    home: None,
                    shell: None,
                },
                default: true,
            }],
        };

        let result = config.get_host_mirror_user();
        assert!(result.is_none());
    }
}
