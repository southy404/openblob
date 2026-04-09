use windows::Win32::Foundation::HWND;
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId};

#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub domain: String,
    pub app_name: String,
    pub process_name: String,
    pub window_title: String,
    pub confidence: f32,
    pub source: String,
}

fn detect_domain(process: &str, title: &str) -> String {
    let p = process.to_lowercase();
    let t = title.to_lowercase();

    if p.contains("chrome")
        || p.contains("msedge")
        || p.contains("firefox")
        || t.contains("youtube")
        || t.contains("netflix")
    {
        return "browser".into();
    }

    if p.contains("code")
        || p.contains("devenv")
        || p.contains("studio")
        || p.contains("idea")
    {
        return "editor".into();
    }

    if p.contains("spotify") || t.contains("spotify") {
        return "media".into();
    }

    if p.ends_with(".exe")
        && !p.contains("explorer")
        && !p.contains("openblob")
        && !p.contains("msedgewebview2")
    {
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
    }
}

pub fn resolve_active_context() -> ActiveContext {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();

        if hwnd.0 == 0 {
            return ActiveContext {
                domain: "desktop".into(),
                app_name: "unknown".into(),
                process_name: "unknown".into(),
                window_title: String::new(),
                confidence: 0.2,
                source: "process".into(),
            };
        }

        let window_title = get_active_window_title(hwnd);

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return ActiveContext {
                domain: "desktop".into(),
                app_name: "unknown".into(),
                process_name: "unknown".into(),
                window_title,
                confidence: 0.2,
                source: "process".into(),
            };
        }

        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);

        if let Ok(process) = process {
            let mut buffer = [0u16; 260];
            let len = K32GetModuleFileNameExW(process, None, &mut buffer);

            if len > 0 {
                let path = String::from_utf16_lossy(&buffer[..len as usize]);
                let process_name = path
                    .split('\\')
                    .last()
                    .unwrap_or("unknown")
                    .to_string();

                let app_name = process_name.clone();
                let domain = detect_domain(&process_name, &window_title);

                return ActiveContext {
                    domain,
                    app_name,
                    process_name,
                    window_title,
                    confidence: 0.75,
                    source: "process".into(),
                };
            }
        }

        ActiveContext {
            domain: "desktop".into(),
            app_name: "unknown".into(),
            process_name: "unknown".into(),
            window_title,
            confidence: 0.2,
            source: "process".into(),
        }
    }
}