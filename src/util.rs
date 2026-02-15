use crate::error::{DevshellError, Diagnosis};
use crate::fs;
use std::process::Command;

pub fn format_diagnosis(diagnosis: &Diagnosis) -> String {
    match diagnosis {
        Diagnosis::FieldMisplacement {
            field,
            wrong_location,
            suggestion,
            file_path,
        } => {
            let mut result = String::new();
            result.push_str(&format!(
                "error: field `{}` found in {}",
                field, wrong_location
            ));

            // Add file context if available
            if let Some(path) = file_path {
                result.push_str(&format!("\n   --> {}", path));
                result.push_str("\n    |\n    |");
            }

            // Add rustc-style help section
            result.push_str(&format!(
                "\n    |\nhelp: field `{}` should be at top level",
                field
            ));
            result.push_str(&format!("\n    |\n     {}", suggestion));
            result
        }
        Diagnosis::MissingRequiredField {
            field,
            reason,
            suggestion,
            file_path,
        } => {
            let mut result = String::new();
            result.push_str(&format!("warning: missing required field `{}`", field));

            if let Some(path) = file_path {
                result.push_str(&format!("\n    --> {}", path));
            }

            result.push_str(&format!("\n    |\nhelp: {}", reason));
            result.push_str(&format!("\n    |\n     {}", suggestion));
            result
        }
        Diagnosis::InconsistentConfiguration {
            issue,
            suggestion,
            file_path,
        } => {
            let mut result = String::new();
            result.push_str(&format!("warning: {}", issue));

            if let Some(path) = file_path {
                result.push_str(&format!("\n    --> {}", path));
            }

            result.push_str(&format!("\n    |\nhelp: {}", suggestion));
            result
        }
        Diagnosis::SyntaxWarning {
            warning,
            suggestion,
            file_path,
        } => {
            let mut result = String::new();
            result.push_str(&format!("info: {}", warning));

            if let Some(path) = file_path {
                result.push_str(&format!("\n    --> {}", path));
            }

            result.push_str(&format!("\n    |\nhelp: {}", suggestion));
            result
        }
    }
}

pub fn run_doctor() -> Result<(), DevshellError> {
    println!("🔍 Devshell System Check\n");

    // Run basic system checks first
    check_docker();
    check_directories();
    check_config_resolution();
    check_fragments();

    println!("\n🩺 Configuration Analysis");

    // Run enhanced configuration diagnostics
    match crate::config::diagnostic::ConfigDiagnostic::diagnose_all() {
        Ok(_) => {
            println!("✅ Configuration analysis completed - no issues found");
        }
        Err(DevshellError::ConfigurationDiagnosis(diagnoses)) => {
            // Display all configuration issues with rustc-style formatting
            for (i, diagnosis) in diagnoses.iter().enumerate() {
                println!("\n{}: {}", i + 1, format_diagnosis(diagnosis));
            }

            // Summary
            let errors = diagnoses
                .iter()
                .filter(|d| d.severity() == crate::error::Severity::Error)
                .count();
            let warnings = diagnoses
                .iter()
                .filter(|d| d.severity() == crate::error::Severity::Warning)
                .count();
            let info = diagnoses
                .iter()
                .filter(|d| d.severity() == crate::error::Severity::Info)
                .count();

            println!(
                "\n📊 Summary: {} error(s), {} warning(s), {} info(s)",
                errors, warnings, info
            );

            if errors > 0 {
                println!("\n❌ Configuration errors detected - please fix before continuing");
            } else if warnings > 0 {
                println!("\n⚠️  Configuration warnings detected - recommended to fix");
            }

            // Return the error so doctor fails appropriately
            return Err(DevshellError::ConfigurationDiagnosis(diagnoses));
        }
        Err(e) => {
            // Other errors (IO, TOML, etc.)
            return Err(e);
        }
    }

    println!("\n✅ Doctor check completed");
    Ok(())
}

fn check_docker() {
    println!("🐳 Checking Docker...");

    match Command::new("docker").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("   ✅ Docker installed: {}", version.trim());
        }
        _ => {
            println!("   ❌ Docker not found or not working");
            return;
        }
    }

    match Command::new("docker").arg("info").output() {
        Ok(output) if output.status.success() => {
            println!("   ✅ Docker daemon running");
        }
        _ => {
            println!("   ❌ Docker daemon not running");
        }
    }
}

fn check_directories() {
    println!("\n📁 Checking directories...");

    let config_dir = fs::get_config_dir();
    if config_dir.exists() {
        println!("   ✅ Config directory exists: {}", config_dir.display());
    } else {
        println!("   ⚠️  Config directory missing: {}", config_dir.display());
    }

    let fragments_dir = fs::get_fragments_dir();
    if fragments_dir.exists() {
        println!(
            "   ✅ Fragments directory exists: {}",
            fragments_dir.display()
        );
    } else {
        println!(
            "   ⚠️  Fragments directory missing: {}",
            fragments_dir.display()
        );
    }
}

fn check_config_resolution() {
    println!("\n📋 Checking config resolution...");

    match fs::discover_disk_fragments() {
        Ok(fragments) => {
            println!("   ✅ Found {} disk fragments", fragments.len());
        }
        Err(_) => {
            println!("   ⚠️  Could not scan for disk fragments");
        }
    }
}

fn check_fragments() {
    println!("\n🧩 Checking fragments...");

    let embedded_fragments = crate::fragments::resolve::list_embedded_fragments();
    println!(
        "   ✅ {} embedded fragments available",
        embedded_fragments.len()
    );

    for fragment in &embedded_fragments {
        println!("      - {}", fragment);
    }

    match fs::discover_disk_fragments() {
        Ok(disk_fragments) if !disk_fragments.is_empty() => {
            println!("   ✅ {} disk fragments found", disk_fragments.len());
            for fragment in &disk_fragments {
                println!("      - {}", fragment);
            }
        }
        Ok(_) => {
            println!("   ℹ️  No disk fragments found");
        }
        Err(_) => {
            println!("   ⚠️  Could not scan for disk fragments");
        }
    }
}
