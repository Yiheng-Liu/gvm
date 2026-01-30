use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;
use std::process::Command;

/// GVM - Go Version Manager
/// A simple tool to manage multiple Go versions, similar to nvm
#[derive(Parser)]
#[command(name = "gvm")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all installed Go versions
    List,
    /// List all available Go versions from go.dev
    ListAll,
    /// Install a specific Go version
    Install {
        /// Version to install (e.g., 1.22.11 or go1.22.11)
        version: String,
    },
    /// Use a specific Go version
    Use {
        /// Version to use (e.g., 1.22.11 or go1.22.11)
        version: String,
    },
}

#[derive(Debug, Deserialize)]
struct GoRelease {
    version: String,
    stable: bool,
}

fn get_go_bin_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join("go")
        .join("bin")
}

fn normalize_version(version: &str) -> String {
    if version.starts_with("go") {
        version.to_string()
    } else {
        format!("go{}", version)
    }
}

fn extract_version_number(version: &str) -> &str {
    version.strip_prefix("go").unwrap_or(version)
}

fn list_installed_versions() -> Vec<String> {
    let bin_dir = get_go_bin_dir();

    if !bin_dir.exists() {
        return Vec::new();
    }

    let mut versions: Vec<String> = fs::read_dir(&bin_dir)
        .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", bin_dir))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            // Match go1.x.x pattern but not just "go"
            if file_name.starts_with("go1.") && !file_name.contains('.') == false {
                Some(file_name)
            } else {
                None
            }
        })
        .collect();

    versions.sort_by(|a, b| {
        let a_ver = extract_version_number(a);
        let b_ver = extract_version_number(b);
        version_compare(a_ver, b_ver)
    });

    versions
}

fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let parse_version = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = s.split('.').collect();
        let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };

    let a_parsed = parse_version(a);
    let b_parsed = parse_version(b);
    a_parsed.cmp(&b_parsed)
}

fn get_current_version() -> Option<String> {
    let bin_dir = get_go_bin_dir();
    let go_link = bin_dir.join("go");

    if go_link.is_symlink() {
        if let Ok(target) = fs::read_link(&go_link) {
            let target_name = target
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            return target_name;
        }
    }
    None
}

fn cmd_list() {
    let versions = list_installed_versions();
    let current = get_current_version();

    if versions.is_empty() {
        println!("{}", "No Go versions installed.".yellow());
        println!(
            "Use {} to install a version.",
            "gvm install <version>".green()
        );
        return;
    }

    println!("{}", "Installed Go versions:".bold());
    for version in versions {
        let version_num = extract_version_number(&version);
        if Some(version.clone()) == current {
            println!("  {} {} {}", "->".green().bold(), version_num.green().bold(), "(current)".dimmed());
        } else {
            println!("     {}", version_num);
        }
    }
}

fn cmd_list_all() {
    println!("{}", "Fetching available Go versions...".dimmed());

    let url = "https://go.dev/dl/?mode=json&include=all";

    let response = reqwest::blocking::get(url);

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                eprintln!("{} Failed to fetch versions: HTTP {}", "Error:".red().bold(), resp.status());
                return;
            }

            match resp.json::<Vec<GoRelease>>() {
                Ok(releases) => {
                    let mut versions: Vec<_> = releases
                        .iter()
                        .map(|r| {
                            let version_num = extract_version_number(&r.version);
                            (version_num.to_string(), r.stable)
                        })
                        .collect();

                    // Remove duplicates and sort
                    versions.dedup_by(|a, b| a.0 == b.0);
                    versions.sort_by(|a, b| version_compare(&b.0, &a.0));

                    let installed = list_installed_versions();
                    let installed_nums: Vec<_> = installed
                        .iter()
                        .map(|v| extract_version_number(v).to_string())
                        .collect();

                    println!("{}", "Available Go versions:".bold());
                    println!("{}", "(stable versions marked with *, installed versions marked with ✓)".dimmed());
                    println!();

                    // Show latest 30 versions by default
                    for (version, stable) in versions.iter().take(30) {
                        let is_installed = installed_nums.contains(version);
                        let stable_marker = if *stable { "*" } else { " " };
                        let install_marker = if is_installed {
                            "✓".green().to_string()
                        } else {
                            " ".to_string()
                        };

                        if *stable {
                            println!("  {} {} {}", install_marker, stable_marker.cyan(), version.cyan());
                        } else {
                            println!("  {} {} {}", install_marker, stable_marker, version);
                        }
                    }

                    println!();
                    println!(
                        "{}",
                        format!("Showing latest 30 of {} versions.", versions.len()).dimmed()
                    );
                }
                Err(e) => {
                    eprintln!("{} Failed to parse response: {}", "Error:".red().bold(), e);
                }
            }
        }
        Err(e) => {
            eprintln!("{} Failed to fetch versions: {}", "Error:".red().bold(), e);
        }
    }
}

fn cmd_install(version: &str) {
    let normalized = normalize_version(version);
    let version_num = extract_version_number(&normalized);

    // Check if already installed
    let bin_dir = get_go_bin_dir();
    let go_wrapper = bin_dir.join(&normalized);

    if go_wrapper.exists() {
        println!(
            "{} Go {} is already installed.",
            "✓".green().bold(),
            version_num.green()
        );
        println!(
            "Use {} to switch to this version.",
            format!("gvm use {}", version_num).cyan()
        );
        return;
    }

    println!(
        "{} {}",
        "Installing Go version:".bold(),
        version_num.green()
    );

    // Step 1: go install golang.org/dl/goX.X.X@latest
    println!("{}", "Step 1/2: Installing Go wrapper...".dimmed());
    let install_pkg = format!("golang.org/dl/{}@latest", normalized);

    let install_result = Command::new("go")
        .args(["install", &install_pkg])
        .status();

    match install_result {
        Ok(status) if status.success() => {
            println!("{}", "  ✓ Go wrapper installed".green());
        }
        Ok(status) => {
            eprintln!(
                "{} go install failed with exit code: {:?}",
                "Error:".red().bold(),
                status.code()
            );
            return;
        }
        Err(e) => {
            eprintln!("{} Failed to run go install: {}", "Error:".red().bold(), e);
            eprintln!(
                "{}",
                "Make sure 'go' is installed and available in your PATH.".yellow()
            );
            return;
        }
    }

    // Step 2: goX.X.X download
    println!("{}", "Step 2/2: Downloading Go SDK...".dimmed());
    let bin_dir = get_go_bin_dir();
    let go_wrapper = bin_dir.join(&normalized);

    if !go_wrapper.exists() {
        eprintln!(
            "{} Go wrapper not found at {:?}",
            "Error:".red().bold(),
            go_wrapper
        );
        return;
    }

    let download_result = Command::new(&go_wrapper).arg("download").status();

    match download_result {
        Ok(status) if status.success() => {
            println!("{}", "  ✓ Go SDK downloaded".green());
            println!();
            println!(
                "{} Go {} installed successfully!",
                "✓".green().bold(),
                version_num.green()
            );
            println!(
                "Use {} to switch to this version.",
                format!("gvm use {}", version_num).cyan()
            );
        }
        Ok(status) => {
            eprintln!(
                "{} download failed with exit code: {:?}",
                "Error:".red().bold(),
                status.code()
            );
        }
        Err(e) => {
            eprintln!("{} Failed to download Go SDK: {}", "Error:".red().bold(), e);
        }
    }
}

fn cmd_use(version: &str) {
    let normalized = normalize_version(version);
    let version_num = extract_version_number(&normalized);

    let bin_dir = get_go_bin_dir();
    let go_wrapper = bin_dir.join(&normalized);
    let go_link = bin_dir.join("go");

    // Check if version is installed
    if !go_wrapper.exists() {
        eprintln!(
            "{} Go {} is not installed.",
            "Error:".red().bold(),
            version_num
        );
        eprintln!(
            "Run {} to install it first.",
            format!("gvm install {}", version_num).cyan()
        );
        return;
    }

    // Remove existing symlink or file
    if go_link.exists() || go_link.is_symlink() {
        if let Err(e) = fs::remove_file(&go_link) {
            eprintln!(
                "{} Failed to remove existing 'go': {}",
                "Error:".red().bold(),
                e
            );
            return;
        }
    }

    // Create new symlink
    match unix_fs::symlink(&go_wrapper, &go_link) {
        Ok(_) => {
            println!(
                "{} Now using Go {}",
                "✓".green().bold(),
                version_num.green()
            );

            // Verify by running go version
            if let Ok(output) = Command::new(&go_link).arg("version").output() {
                if output.status.success() {
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    println!("{}", version_output.trim().dimmed());
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{} Failed to create symlink: {}",
                "Error:".red().bold(),
                e
            );
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => cmd_list(),
        Commands::ListAll => cmd_list_all(),
        Commands::Install { version } => cmd_install(&version),
        Commands::Use { version } => cmd_use(&version),
    }
}
