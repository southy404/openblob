use std::collections::HashMap;

use strsim::jaro_winkler;

#[derive(Debug, Clone)]
pub struct DiscoveredApp {
    pub canonical_name: String,
    pub launch_target: String,
    pub score: f64,
    pub source: String,
}

#[cfg(not(windows))]
pub fn find_app_launch_target(_query: &str) -> Option<DiscoveredApp> {
    None
}

#[cfg(windows)]
mod windows_impl {
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};

    use walkdir::WalkDir;
    use winreg::enums::*;
    use winreg::RegKey;

    use super::{alias_candidates, normalize, score_candidate, DiscoveredApp};

    static APP_CACHE: OnceLock<Mutex<Vec<DiscoveredApp>>> = OnceLock::new();

    fn cache() -> &'static Mutex<Vec<DiscoveredApp>> {
        APP_CACHE.get_or_init(|| Mutex::new(Vec::new()))
    }

    pub fn find_app_launch_target(query: &str) -> Option<DiscoveredApp> {
        let q = normalize(query);

        let aliases = alias_candidates();
        let mut candidates: Vec<DiscoveredApp> = Vec::new();

        for (name, launch) in aliases {
            let score = score_candidate(&q, &normalize(name));
            if score >= 0.80 || normalize(name).contains(&q) || q.contains(&normalize(name)) {
                candidates.push(DiscoveredApp {
                    canonical_name: name.to_string(),
                    launch_target: launch.to_string(),
                    score,
                    source: "alias".into(),
                });
            }
        }

        {
            let cached = cache().lock().ok()?.clone();
            for app in cached {
                let score = score_candidate(&q, &normalize(&app.canonical_name));
                if score >= 0.80
                    || normalize(&app.canonical_name).contains(&q)
                    || q.contains(&normalize(&app.canonical_name))
                {
                    candidates.push(DiscoveredApp { score, ..app });
                }
            }
        }

        if let Some(found) = scan_registry_and_start_menu_best(&q) {
            candidates.push(found);
        }

        if let Some(found) = scan_program_files_best(&q) {
            candidates.push(found);
        }

        if let Some(found) = path_match_best(&q) {
            candidates.push(found);
        }

        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        let best = candidates.into_iter().next()?;

        remember(best.clone());
        Some(best)
    }

    fn remember(app: DiscoveredApp) {
        if let Ok(mut c) = cache().lock() {
            let exists = c.iter().any(|x| {
                normalize(&x.canonical_name) == normalize(&app.canonical_name)
                    && x.launch_target == app.launch_target
            });
            if !exists {
                c.push(app);
            }
        }
    }

    fn scan_registry_and_start_menu_best(query: &str) -> Option<DiscoveredApp> {
        let mut found = Vec::new();

        found.extend(scan_registry(query));
        found.extend(scan_start_menu(query));

        found.sort_by(|a, b| b.score.total_cmp(&a.score));
        found.into_iter().next()
    }

    fn scan_registry(query: &str) -> Vec<DiscoveredApp> {
        let mut results = Vec::new();

        let hives = [
            RegKey::predef(HKEY_LOCAL_MACHINE),
            RegKey::predef(HKEY_CURRENT_USER),
        ];

        for hive in hives {
            if let Ok(app_paths) = hive.open_subkey(
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths",
            ) {
                for sub in app_paths.enum_keys().flatten() {
                    let sub_norm = normalize(&sub);
                    let score = score_candidate(query, &sub_norm);

                    if score < 0.78 && !sub_norm.contains(query) && !query.contains(&sub_norm) {
                        continue;
                    }

                    if let Ok(subkey) = app_paths.open_subkey(&sub) {
                        let value: Result<String, _> = subkey.get_value("");
                        if let Ok(path) = value {
                            results.push(DiscoveredApp {
                                canonical_name: sub.clone(),
                                launch_target: path,
                                score,
                                source: "registry".into(),
                            });
                        }
                    }
                }
            }
        }

        results
    }

    fn start_menu_roots() -> Vec<PathBuf> {
        let mut roots = vec![PathBuf::from(
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
        )];

        if let Ok(appdata) = std::env::var("APPDATA") {
            roots.push(
                PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"),
            );
        }

        roots
    }

    fn scan_start_menu(query: &str) -> Vec<DiscoveredApp> {
        let mut results = Vec::new();

        for root in start_menu_roots() {
            if !root.exists() {
                continue;
            }

            for entry in WalkDir::new(root)
                .max_depth(5)
                .into_iter()
                .filter_map(Result::ok)
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_lowercase();

                if ext != "lnk" && ext != "url" {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();

                let name_norm = normalize(&name);
                let score = score_candidate(query, &name_norm);

                if score < 0.78 && !name_norm.contains(query) && !query.contains(&name_norm) {
                    continue;
                }

                results.push(DiscoveredApp {
                    canonical_name: name,
                    launch_target: path.display().to_string(),
                    score,
                    source: "start-menu".into(),
                });
            }
        }

        results
    }

    fn scan_program_files_best(query: &str) -> Option<DiscoveredApp> {
        let mut candidates = Vec::new();
        for root in program_files_roots() {
            if !root.exists() {
                continue;
            }
            for entry in WalkDir::new(root)
                .max_depth(4)
                .into_iter()
                .filter_map(Result::ok)
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if ext != "exe" {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                let score = score_candidate(query, &normalize(&name));
                if score < 0.82 {
                    continue;
                }

                candidates.push(DiscoveredApp {
                    canonical_name: name,
                    launch_target: path.display().to_string(),
                    score,
                    source: "program-files".into(),
                });
            }
        }

        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates.into_iter().next()
    }

    fn program_files_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(p) = std::env::var("ProgramFiles") {
            roots.push(PathBuf::from(p));
        }
        if let Ok(p) = std::env::var("ProgramFiles(x86)") {
            roots.push(PathBuf::from(p));
        }
        roots
    }

    fn path_match_best(query: &str) -> Option<DiscoveredApp> {
        let Ok(path) = std::env::var("PATH") else {
            return None;
        };

        let mut candidates = Vec::new();
        for dir in path.split(';').filter(|s| !s.trim().is_empty()) {
            let p = Path::new(dir);
            if !p.exists() {
                continue;
            }
            for entry in std::fs::read_dir(p).ok()?.flatten() {
                let file_path = entry.path();
                if !file_path.is_file() {
                    continue;
                }
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if ext != "exe" {
                    continue;
                }
                let name = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                let score = score_candidate(query, &normalize(&name));
                if score < 0.86 {
                    continue;
                }
                candidates.push(DiscoveredApp {
                    canonical_name: name,
                    launch_target: file_path.display().to_string(),
                    score,
                    source: "path".into(),
                });
            }
        }

        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates.into_iter().next()
    }
}

#[cfg(windows)]
pub use windows_impl::find_app_launch_target;

fn normalize(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
}

fn score_candidate(query: &str, candidate: &str) -> f64 {
    if query == candidate {
        return 1.0;
    }
    if candidate.contains(query) || query.contains(candidate) {
        return 0.95;
    }
    jaro_winkler(query, candidate)
}

fn alias_candidates() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("taschenrechner", "calc"),
        ("rechner", "calc"),
        ("calculator", "calc"),
        ("calc", "calc"),
        ("paint", "mspaint"),
        ("mspaint", "mspaint"),
        ("notepad", "notepad"),
        ("editor", "notepad"),
        ("explorer", "explorer"),
        ("datei explorer", "explorer"),
        ("file explorer", "explorer"),
        ("taskmanager", "taskmgr"),
        ("task manager", "taskmgr"),
        ("settings", "ms-settings:"),
        ("einstellungen", "ms-settings:"),
        ("steam", "steam"),
        ("discord", "discord"),
        ("spotify", "spotify"),
        ("chrome", "chrome"),
        ("edge", "msedge"),
        ("fl studio", "FL64"),
        ("fl", "FL64"),
        ("obs", "obs64"),
        ("visual studio code", "code"),
        ("vscode", "code"),
    ])
}
