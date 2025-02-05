use core::str;
use esp_metadata::Chip;
use reqwest::header;
use std::{env, error::Error, process};

/// Xtensa Rust Toolchain API URL
const XTENSA_RUST_LATEST_API_URL: &str =
    "https://api.github.com/repos/esp-rs/rust-build/releases/latest";
const ESPFLASH_LATEST_API_URL: &str =
    "https://api.github.com/repos/esp-rs/espflash/releases/latest";
const PROBE_RS_LATEST_API_URL: &str =
    "https://api.github.com/repos/probe-rs/probe-rs/releases/latest";
const STABLE_RUST_LATEST_API_URL: &str =
    "https://api.github.com/repos/rust-lang/rust/releases/latest";
const ESP_HAL_LATEST_API_URL: &str = "https://api.github.com/repos/esp-rs/esp-hal/releases/latest";

struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

enum CheckResult {
    Ok,
    WrongVersion,
    NotFound,
}

pub fn check(chip: Chip) {
    let rust_version = get_version(
        "cargo",
        if chip.is_xtensa() {
            &["+esp"]
        } else {
            &["+stable"]
        },
    );

    let espflash_version = get_version("espflash", &[]);
    let probers_version = get_version("probe-rs", &[]);

    let xtensa_rust = get_latest_version(XTENSA_RUST_LATEST_API_URL).unwrap();
    println!("Latest Xtensa Rust version: {}", xtensa_rust);
    let stable_rust = get_latest_version(STABLE_RUST_LATEST_API_URL).unwrap();
    println!("Latest STABLE Rust version: {}", stable_rust);
    let espflash = get_latest_version(ESPFLASH_LATEST_API_URL).unwrap();
    println!("Latest espflash version: {}", espflash);
    let probers = get_latest_version(PROBE_RS_LATEST_API_URL).unwrap();
    println!("Latest probe-rs version: {}", probers);
    let esp_hal = get_latest_version(ESP_HAL_LATEST_API_URL).unwrap();
    println!("Latest esp_hal version: {}", esp_hal);

    println!("\nChecking installed versions");
    print_result("Rust", check_version(rust_version, 1, 84, 0));
    print_result("espflash", check_version(espflash_version, 3, 3, 0));
    print_result("probe-rs", check_version(probers_version, 0, 25, 0));
}

fn print_result(name: &str, check_result: CheckResult) {
    match check_result {
        CheckResult::Ok => println!("🆗 {}", name),
        CheckResult::WrongVersion => println!("🛑 {}", name),
        CheckResult::NotFound => println!("❌ {}", name),
    }
}

fn check_version(version: Option<Version>, major: u8, minor: u8, patch: u8) -> CheckResult {
    match version {
        Some(version) => {
            if version.major >= major && version.minor >= minor && version.patch >= patch {
                CheckResult::Ok
            } else {
                CheckResult::WrongVersion
            }
        }
        None => CheckResult::NotFound,
    }
}

fn get_version(cmd: &str, args: &[&str]) -> Option<Version> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .arg("--version")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                if let Ok(output) = str::from_utf8(&output.stdout) {
                    let mut parts = output.split_whitespace();
                    let _name = parts.next();
                    let version = parts.next();
                    if let Some(version) = version {
                        let mut version = version.split(&['.', '-', '+']);
                        let major = version.next().unwrap().parse::<u8>().unwrap();
                        let minor = version.next().unwrap().parse::<u8>().unwrap();
                        let patch = version.next().unwrap().parse::<u8>().unwrap();
                        return Some(Version {
                            major,
                            minor,
                            patch,
                        });
                    }
                }
            }

            None
        }
        Err(_) => None,
    }
}

/// Get the latest version of Xtensa Rust toolchain.
pub fn get_latest_version(url: &str) -> Result<String, Box<dyn Error>> {
    log::debug!("Querying GitHub API: '{}'", url);
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, "esp-genereate".parse().unwrap());
    headers.insert(
        header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );

    headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
    if let Some(token) = env::var_os("GITHUB_TOKEN") {
        log::debug!("Auth header added");
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.to_string_lossy())
                .parse()
                .unwrap(),
        );
    }
    let client = reqwest::blocking::Client::builder().build()?;
    let res = client.get(url).headers(headers.clone()).send()?.text()?;
    if res.contains("https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting")
    {
        log::error!("API Rate Limit Exceeded");
        process::exit(-1);
    }

    if res.contains("Bad credentials") {
        log::error!("Invalid GitHub token");
        process::exit(-1);
    }

    let json: serde_json::Value = serde_json::from_str(&res)?;

    let mut version = json["tag_name"].to_string();

    version.retain(|c| c != 'v' && c != '"');

    Ok(version)
}
