use crate::error::DevshellError;
use crate::fs;
use std::process::Command;

pub fn run_doctor() -> Result<(), DevshellError> {
    println!("🔍 Devshell System Check\n");

    check_docker();
    check_directories();
    check_config_resolution();
    check_fragments();

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
