//
// johsve-source@github.com
// 2025-08-22
//

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use ansi_term::Colour::RGB;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PackageInfo {
    date: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CacheData {
    pkg_hash: String,
    last_log_size: u64,
    data: HashMap<String, PackageInfo>,
}

fn get_log_size() -> u64 {
    fs::metadata("/var/log/pacman.log")
        .map(|m| m.len())
        .unwrap_or(0)
}

fn parse_log_entries(log_content: &str) -> HashMap<String, PackageInfo> {
    let re =
        Regex::new(r"\[([0-9T:+-]+)\] \[ALPM\] (installed|upgraded|removed) ([^\s(]+)").unwrap();

    log_content
        .par_lines()
        .filter_map(|line| {
            re.captures(line).map(|caps| {
                let date_str = caps.get(1).unwrap().as_str().to_string();
                let action = caps.get(2).unwrap().as_str();
                let pkg_name = caps.get(3).unwrap().as_str().to_string();
                let status = match action {
                    "installed" => "INS".to_string(),
                    "upgraded" => "UPG".to_string(),
                    "removed" => "REM".to_string(),
                    _ => "ERR".to_string(),
                };
                (
                    pkg_name,
                    PackageInfo {
                        date: date_str,
                        status,
                    },
                )
            })
        })
        .collect()
}

fn main() {
    let date_color = RGB(203, 166, 247);
    let pkg_color = RGB(137, 180, 250);
    let ins_color = RGB(166, 227, 161);
    let upg_color = RGB(249, 226, 175);
    let rem_color = RGB(250, 179, 135);
    let err_color = RGB(243, 139, 168);

    let cache_file = "/tmp/pkglist_cache.json";

    // Aktuella installerade paket
    let output = Command::new("pacman")
        .args(["-Qeq"])
        .output()
        .expect("failed to run pacman");
    let current_pkgs: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    let current_pkg_hash = format!("{:x}", md5::compute(current_pkgs.join(",")));
    let current_log_size = get_log_size();

    // Läs cache
    let mut cache_data: CacheData = if Path::new(cache_file).exists() {
        fs::read_to_string(cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(CacheData {
                pkg_hash: String::new(),
                last_log_size: 0,
                data: HashMap::new(),
            })
    } else {
        CacheData {
            pkg_hash: String::new(),
            last_log_size: 0,
            data: HashMap::new(),
        }
    };

    // Uppdatera cache om logg eller paketlista ändrats
    if cache_data.pkg_hash != current_pkg_hash || cache_data.last_log_size != current_log_size {
        if let Ok(log_content) = fs::read_to_string("/var/log/pacman.log") {
            cache_data.data = parse_log_entries(&log_content);
            cache_data.last_log_size = current_log_size;
        }
        cache_data.pkg_hash = current_pkg_hash;
        let _ = fs::write(
            cache_file,
            serde_json::to_string_pretty(&cache_data).unwrap(),
        );
    }

    // Kombinera installerade paket med tidigare loggade paket (inkl. REM)
    let mut pkg_set: HashMap<String, (String, String)> = cache_data
        .data
        .iter()
        .map(|(k, v)| (k.clone(), (v.date.clone(), v.status.clone())))
        .collect();

    for pkg in &current_pkgs {
        pkg_set.insert(
            pkg.clone(),
            pkg_set
                .get(pkg)
                .cloned()
                .unwrap_or(("0000-00-00T00:00:00+0000".to_string(), "INS".to_string())),
        );
    }

    // Konvertera till vektor och sortera på datum
    let mut pkg_list: Vec<(String, String, String)> = pkg_set
        .into_iter()
        .map(|(pkg, (date, status))| (pkg, date, status))
        .collect();
    pkg_list.sort_by(|a, b| a.1.cmp(&b.1));

    // Skriv ut
    for (pkg, date_str, status) in pkg_list {
        let status_colored = match status.as_str() {
            "INS" => ins_color.paint(&status),
            "UPG" => upg_color.paint(&status),
            "REM" => rem_color.paint(&status),
            _ => err_color.paint(&status),
        };

        println!(
            "{} :: {} :: {}",
            date_color.paint(date_str),
            status_colored,
            pkg_color.paint(pkg)
        );
    }
}
