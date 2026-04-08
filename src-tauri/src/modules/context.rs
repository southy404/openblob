use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::Foundation::HWND;

#[tauri::command]
pub fn get_active_app() -> String {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();

        if hwnd.0 == 0 {
            return "unknown".into();
        }

        let mut pid: u32 = 0;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return "unknown".into();
        }

        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);

        if process.is_err() {
            return "unknown".into();
        }

        let process = process.unwrap();

        let mut buffer = [0u16; 260];

        let len = K32GetModuleFileNameExW(
            process,
            None,
            &mut buffer,
        );

        if len == 0 {
            return "unknown".into();
        }

        let path = String::from_utf16_lossy(&buffer[..len as usize]);

        path.split('\\').last().unwrap_or("unknown").to_string()
    }
}