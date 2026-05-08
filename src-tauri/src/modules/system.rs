use std::process::Command;

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN, VK_VOLUME_MUTE,
    VK_VOLUME_UP,
};

#[cfg(windows)]
fn send_vk(vk: VIRTUAL_KEY) -> Result<(), String> {
    unsafe {
        let down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let inputs = [down, up];
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);

        if sent == 0 {
            return Err("SendInput failed.".into());
        }

        Ok(())
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn send_vk(_vk: ()) -> Result<(), String> {
    Err("Not supported on this OS.".into())
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| format!("Failed to run AppleScript: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "AppleScript command failed.".into()
        } else {
            stderr
        })
    }
}

#[cfg(target_os = "macos")]
fn macos_key_code(code: u16) -> Result<(), String> {
    run_osascript(&format!(
        "tell application \"System Events\" to key code {code}"
    ))
    .map(|_| ())
}

#[tauri::command]
pub fn media_play_pause() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_MEDIA_PLAY_PAUSE);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(100)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn media_next_track() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_MEDIA_NEXT_TRACK);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(101)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn media_prev_track() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_MEDIA_PREV_TRACK);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(98)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn volume_key_up() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_VOLUME_UP);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(111)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn volume_key_down() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_VOLUME_DOWN);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(103)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn volume_key_mute() -> Result<(), String> {
    #[cfg(windows)]
    {
        return send_vk(VK_VOLUME_MUTE);
    }

    #[cfg(target_os = "macos")]
    {
        macos_key_code(109)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return send_vk(());
    }
}

#[tauri::command]
pub fn get_system_volume() -> Result<f32, String> {
    #[cfg(target_os = "macos")]
    {
        let value = run_osascript("output volume of (get volume settings)")?;
        let percent = value
            .parse::<f32>()
            .map_err(|e| format!("Failed to parse macOS volume: {e}"))?;
        Ok((percent / 100.0).clamp(0.0, 1.0))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Reading exact system volume is not implemented yet.".into())
    }
}

#[tauri::command]
pub fn set_system_volume(value: f32) -> Result<f32, String> {
    #[cfg(target_os = "macos")]
    {
        let percent = (value.clamp(0.0, 1.0) * 100.0).round() as u8;
        run_osascript(&format!("set volume output volume {percent}"))?;
        Ok(percent as f32 / 100.0)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = value;
        Err("Setting exact system volume percent is not implemented yet.".into())
    }
}

#[tauri::command]
pub fn change_system_volume(delta: f32) -> Result<f32, String> {
    #[cfg(windows)]
    {
        if delta > 0.0 {
            send_vk(VK_VOLUME_UP)?;
        } else if delta < 0.0 {
            send_vk(VK_VOLUME_DOWN)?;
        }

        return Ok(0.0);
    }

    #[cfg(target_os = "macos")]
    {
        let current = get_system_volume().unwrap_or(0.0);
        set_system_volume(current + delta)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = delta;
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn get_system_mute() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let value = run_osascript("output muted of (get volume settings)")?;
        Ok(matches!(value.trim(), "true" | "yes"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Reading exact mute state is not implemented yet.".into())
    }
}

#[tauri::command]
pub fn set_system_mute(value: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        send_vk(VK_VOLUME_MUTE)?;
        return Ok(value);
    }

    #[cfg(target_os = "macos")]
    {
        run_osascript(&format!(
            "set volume {}",
            if value {
                "with output muted"
            } else {
                "without output muted"
            }
        ))?;
        Ok(value)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = value;
        return Err("Not supported on this OS.".into());
    }
}

#[tauri::command]
pub fn toggle_system_mute() -> Result<bool, String> {
    #[cfg(windows)]
    {
        send_vk(VK_VOLUME_MUTE)?;
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let next = !get_system_mute().unwrap_or(false);
        set_system_mute(next)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return Err("Not supported on this OS.".into());
    }
}

#[tauri::command]
pub fn open_downloads() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg("shell:Downloads")
            .spawn()
            .map_err(|e| format!("Failed to open Downloads: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let downloads = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/"))
            .join("Downloads");

        Command::new("open")
            .arg(downloads)
            .spawn()
            .map_err(|e| format!("Failed to open Downloads: {e}"))?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn open_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg("ms-settings:")
            .spawn()
            .map_err(|e| format!("Failed to open Settings: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "System Settings"])
            .spawn()
            .or_else(|_| {
                Command::new("open")
                    .args(["-b", "com.apple.systempreferences"])
                    .spawn()
            })
            .map_err(|e| format!("Failed to open System Settings: {e}"))?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn open_explorer() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .spawn()
            .map_err(|e| format!("Failed to open File Explorer: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Finder"])
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {e}"))?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn lock_screen() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Shutdown::LockWorkStation;

        unsafe { LockWorkStation() }.map_err(|e| format!("Failed to lock screen: {e}"))?;

        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("pmset")
            .arg("displaysleepnow")
            .spawn()
            .map_err(|e| format!("Failed to lock screen: {e}"))?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn shutdown_pc() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("shutdown")
            .args(["/s", "/t", "0"])
            .spawn()
            .map_err(|e| format!("Failed to shut down PC: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        run_osascript("tell application \"System Events\" to shut down")?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}

#[tauri::command]
pub fn restart_pc() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("shutdown")
            .args(["/r", "/t", "0"])
            .spawn()
            .map_err(|e| format!("Failed to restart PC: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        run_osascript("tell application \"System Events\" to restart")?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Err("Not supported on this OS.".into())
    }
}
