use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SteamGame {
    pub appid: String,
    pub name: String,
}

pub fn find_steam_game(query: &str) -> Option<SteamGame> {
    let games = load_all_games().ok()?;
    let q = normalize(query);

    if q.len() < 3 {
        return None;
    }

    let mut best: Option<SteamGame> = None;
    let mut best_score = 0usize;

    for game in games {
        let name = normalize(&game.name);

        let score = if name == q {
            1000
        } else if name.contains(&q) || q.contains(&name) {
            850
        } else {
            q.split_whitespace()
                .filter(|w| w.len() >= 3 && name.contains(w))
                .count()
                * 120
        };

        if score > best_score {
            best_score = score;
            best = Some(game);
        }
    }

    if best_score >= 240 {
        best
    } else {
        None
    }
}

pub fn steam_launch_uri(appid: &str) -> String {
    format!("steam://rungameid/{}", appid)
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
}

fn load_all_games() -> Result<Vec<SteamGame>, String> {
    let steam_root = detect_steam_root().ok_or("Steam nicht gefunden.")?;
    let mut libraries = vec![steam_root.join("steamapps")];

    let libraryfolders = steam_root.join("steamapps").join("libraryfolders.vdf");
    if libraryfolders.exists() {
        let text = std::fs::read_to_string(&libraryfolders).map_err(|e| e.to_string())?;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('"') && trimmed.contains("\\\\") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    let candidate = parts[3].replace("\\\\", "\\");
                    libraries.push(PathBuf::from(candidate).join("steamapps"));
                }
            }
        }
    }

    let mut games = Vec::new();

    for lib in libraries {
        if !lib.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&lib) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                    continue;
                }

                if let Ok(text) = std::fs::read_to_string(&path) {
                    let appid = extract_vdf_value(&text, "appid").unwrap_or_default();
                    let game_name = extract_vdf_value(&text, "name").unwrap_or_default();

                    if !appid.is_empty() && !game_name.is_empty() {
                        games.push(SteamGame {
                            appid,
                            name: game_name,
                        });
                    }
                }
            }
        }
    }

    Ok(games)
}

fn detect_steam_root() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"D:\Steam",
    ];

    for c in candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    None
}

fn extract_vdf_value(text: &str, key: &str) -> Option<String> {
    let mut waiting_for_value = false;

    for part in text.split('"').map(str::trim) {
        if part.is_empty() {
            continue;
        }

        if waiting_for_value {
            return Some(part.to_string());
        }

        if part == key {
            waiting_for_value = true;
        }
    }

    None
}