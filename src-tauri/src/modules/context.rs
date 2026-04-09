use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};

#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub domain: String,
    pub app_name: String,
    pub process_name: String,
    pub window_title: String,
    pub confidence: f32,
    pub source: String,
}


fn unknown_context(window_title: String) -> ActiveContext {
    ActiveContext {
        domain: "desktop".into(),
        app_name: "unknown".into(),
        process_name: "unknown".into(),
        window_title,
        confidence: 0.2,
        source: "process".into(),
    }
}

pub fn is_internal_companion_app(app: &str) -> bool {
    let lower = app.trim().to_lowercase();

    lower.is_empty()
        || lower == "unknown"
        || lower.contains("openblob")
        || lower.contains("companion")
        || lower.contains("openblob")
        || lower.contains("webview")
        || lower.contains("msedgewebview2")
        || lower.contains("snip-panel")
        || lower.contains("snip-overlay")
        || lower.contains("bubble")
        || lower.contains("speech")
}

fn detect_domain(process: &str, title: &str) -> String {
    let p = process.to_lowercase();
    let t = title.to_lowercase();

    if p.contains("chrome")
        || p.contains("msedge")
        || p.contains("firefox")
        || p.contains("opera")
        || p.contains("brave")
        || t.contains("youtube")
        || t.contains("netflix")
        || t.contains("twitch")
    {
        return "browser".into();
    }

    if p.contains("code")
        || p.contains("codium")
        || p.contains("devenv")
        || p.contains("studio")
        || p.contains("idea")
        || p.contains("pycharm")
        || p.contains("webstorm")
        || p.contains("notepad++")
    {
        return "editor".into();
    }

    if p.contains("spotify")
        || p.contains("vlc")
        || p.contains("foobar")
        || p.contains("music")
        || t.contains("spotify")
    {
        return "media".into();
    }

    if is_internal_companion_app(&p) {
        return "companion".into();
    }

    if p.ends_with(".exe") && !p.contains("explorer") && !p.contains("dwm") {
        return "game".into();
    }

    "desktop".into()
}

fn get_active_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }

        let mut buffer = vec![0u16; (len + 1) as usize];
        let read = GetWindowTextW(hwnd, &mut buffer);

        if read <= 0 {
            return String::new();
        }

        String::from_utf16_lossy(&buffer[..read as usize])
            .trim()
            .to_string()
    }
}

fn get_process_name_from_hwnd(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer = [0u16; 260];
        let len = K32GetModuleFileNameExW(process, None, &mut buffer);

        let result = if len > 0 {
            let path = String::from_utf16_lossy(&buffer[..len as usize]);
            Some(
                path.split('\\')
                    .last()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string(),
            )
        } else {
            None
        };

        let _ = CloseHandle(process);
        result
    }
}

fn normalize_app_name(process_name: &str, window_title: &str) -> String {
    let process = process_name.trim();
    let title = window_title.trim();

    if process.is_empty() {
        return "unknown".into();
    }

    let lower_process = process.to_lowercase();
    let lower_title = title.to_lowercase();

    if lower_process.contains("pioneergame") {
        return "ARC Raiders".into();
    }

    if is_internal_companion_app(process) {
        return process.to_string();
    }

    if !title.is_empty() {
        if lower_title.contains("arc raiders") {
            return "ARC Raiders".into();
        }
        if lower_title.contains("netflix") {
            return "Netflix".into();
        }
        if lower_title.contains("youtube") {
            return "YouTube".into();
        }
        if lower_title.contains("spotify") {
            return "Spotify".into();
        }
    }

    process.replace(".exe", "")
}

pub fn resolve_active_context() -> ActiveContext {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();

        if hwnd.0 == 0 {
            return unknown_context(String::new());
        }

        let window_title = get_active_window_title(hwnd);

        let Some(process_name) = get_process_name_from_hwnd(hwnd) else {
            return unknown_context(window_title);
        };

        let app_name = normalize_app_name(&process_name, &window_title);
        let domain = detect_domain(&process_name, &window_title);

        let confidence = if app_name == "unknown" {
            0.2
        } else if is_internal_companion_app(&app_name) {
            0.55
        } else if !window_title.is_empty() {
            0.9
        } else {
            0.75
        };

        ActiveContext {
            domain,
            app_name,
            process_name,
            window_title,
            confidence,
            source: "process".into(),
        }
    }
}