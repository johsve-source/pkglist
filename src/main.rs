//
// johsve-source@github.com
// 2025-08-28
//

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

use ansi_term::Colour::RGB;
use lazy_static::lazy_static;
use memchr::memchr;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PackageInfo {
    date: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CacheData {
    pkg_hash: u64,
    last_log_size: u64,
    data: HashMap<String, PackageInfo>,
}

lazy_static! {
    static ref LOG_REGEX: Regex =
        Regex::new(r"\[([0-9T:+-]+)\] \[ALPM\] (installed|upgraded|removed) ([^\s(]+)").unwrap();
}

fn get_log_size() -> u64 {
    fs::metadata("/var/log/pacman.log")
        .map(|m| m.len())
        .unwrap_or(0)
}

fn calculate_pkg_hash(pkgs: &[String]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pkgs.hash(&mut hasher);
    hasher.finish()
}

fn parse_log_entries(log_content: &[u8]) -> HashMap<String, PackageInfo> {
    let mut map = HashMap::new();
    let mut pos = 0;

    while let Some(newline_pos) = memchr(b'\n', &log_content[pos..]) {
        let line_end = pos + newline_pos;
        let line = &log_content[pos..line_end];
        pos = line_end + 1;

        if line.len() < 50 {
            continue;
        }

        if let Some(caps) = LOG_REGEX.captures(std::str::from_utf8(line).unwrap_or("")) {
            let date_str = caps.get(1).unwrap().as_str();
            let action = caps.get(2).unwrap().as_str();
            let pkg_name = caps.get(3).unwrap().as_str();

            let status = match action {
                "installed" => "INS".to_string(),
                "upgraded" => "UPG".to_string(),
                "removed" => "REM".to_string(),
                _ => continue,
            };

            map.insert(
                pkg_name.to_string(),
                PackageInfo {
                    date: date_str.to_string(),
                    status,
                },
            );
        }
    }
    map
}

fn read_current_packages() -> Vec<String> {
    Command::new("pacman")
        .args(["-Qeq"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn load_cache(cache_file: &Path) -> Option<CacheData> {
    fs::read(cache_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn save_cache(cache_file: &Path, data: &CacheData) -> io::Result<()> {
    let temp_file = cache_file.with_extension("tmp");
    fs::write(&temp_file, serde_json::to_vec(data)?)?;
    fs::rename(temp_file, cache_file)?;
    Ok(())
}

fn read_log_file() -> io::Result<Vec<u8>> {
    let mut file = fs::File::open("/var/log/pacman.log")?;
    let metadata = file.metadata()?;
    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn main() -> io::Result<()> {
    let date_color = RGB(203, 166, 247);
    let pkg_color = RGB(137, 180, 250);
    let ins_color = RGB(166, 227, 161);
    let upg_color = RGB(249, 226, 175);
    let rem_color = RGB(250, 179, 135);
    let err_color = RGB(243, 139, 168);

    let cache_file = Path::new("/tmp/pkglist_cache.json");
    let current_pkgs = read_current_packages();

    if current_pkgs.is_empty() {
        return Ok(());
    }

    let current_pkg_hash = calculate_pkg_hash(&current_pkgs);
    let current_log_size = get_log_size();

    let mut cache_data = match load_cache(cache_file) {
        Some(data)
            if data.pkg_hash == current_pkg_hash && data.last_log_size == current_log_size =>
        {
            data
        }
        _ => {
            let log_content = read_log_file().unwrap_or_default();
            let parsed_data = parse_log_entries(&log_content);

            CacheData {
                pkg_hash: current_pkg_hash,
                last_log_size: current_log_size,
                data: parsed_data,
            }
        }
    };

    if cache_data.pkg_hash != current_pkg_hash || cache_data.last_log_size != current_log_size {
        let log_content = read_log_file().unwrap_or_default();
        cache_data.data = parse_log_entries(&log_content);
        cache_data.last_log_size = current_log_size;
        cache_data.pkg_hash = current_pkg_hash;

        let _ = save_cache(cache_file, &cache_data);
    }

    let mut pkg_set = HashMap::with_capacity(cache_data.data.len() + current_pkgs.len());

    for (pkg, info) in &cache_data.data {
        pkg_set.insert(pkg.clone(), (info.date.clone(), info.status.clone()));
    }

    for pkg in &current_pkgs {
        pkg_set
            .entry(pkg.clone())
            .or_insert_with(|| ("0000-00-00T00:00:00+0000".to_string(), "INS".to_string()));
    }

    let mut pkg_list: Vec<_> = pkg_set.into_iter().collect();
    pkg_list.sort_unstable_by(|(_, (date1, _)), (_, (date2, _))| date1.cmp(date2));

    for (pkg, (date, status)) in pkg_list {
        let status_colored = match status.as_str() {
            "INS" => ins_color.paint(&status),
            "UPG" => upg_color.paint(&status),
            "REM" => rem_color.paint(&status),
            _ => err_color.paint(&status),
        };

        println!(
            "{} :: {} :: {}",
            date_color.paint(date),
            status_colored,
            pkg_color.paint(pkg)
        );
    }

    Ok(())
}
