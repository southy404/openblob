use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, VIRTUAL_KEY, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN,
    VK_VOLUME_MUTE, VK_VOLUME_UP,
};

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
            return Err("SendInput fehlgeschlagen.".into());
        }

        Ok(())
    }
}

#[tauri::command]
pub fn media_play_pause() -> Result<(), String> {
    send_vk(VK_MEDIA_PLAY_PAUSE)
}

#[tauri::command]
pub fn media_next_track() -> Result<(), String> {
    send_vk(VK_MEDIA_NEXT_TRACK)
}

#[tauri::command]
pub fn media_prev_track() -> Result<(), String> {
    send_vk(VK_MEDIA_PREV_TRACK)
}

#[tauri::command]
pub fn volume_key_up() -> Result<(), String> {
    send_vk(VK_VOLUME_UP)
}

#[tauri::command]
pub fn volume_key_down() -> Result<(), String> {
    send_vk(VK_VOLUME_DOWN)
}

#[tauri::command]
pub fn volume_key_mute() -> Result<(), String> {
    send_vk(VK_VOLUME_MUTE)
}

#[tauri::command]
pub fn get_system_volume() -> Result<f32, String> {
    Err("Systemlautstärke lesen ist noch nicht exakt implementiert.".into())
}

#[tauri::command]
pub fn set_system_volume(_value: f32) -> Result<f32, String> {
    Err("Exakte Prozent-Lautstärke ist noch nicht implementiert.".into())
}

#[tauri::command]
pub fn change_system_volume(delta: f32) -> Result<f32, String> {
    if delta > 0.0 {
        send_vk(VK_VOLUME_UP)?;
    } else if delta < 0.0 {
        send_vk(VK_VOLUME_DOWN)?;
    }

    Ok(0.0)
}

#[tauri::command]
pub fn get_system_mute() -> Result<bool, String> {
    Err("Mute-Status lesen ist noch nicht exakt implementiert.".into())
}

#[tauri::command]
pub fn set_system_mute(value: bool) -> Result<bool, String> {
    // Aktuell nur Toggle möglich
    send_vk(VK_VOLUME_MUTE)?;
    Ok(value)
}

#[tauri::command]
pub fn toggle_system_mute() -> Result<bool, String> {
    send_vk(VK_VOLUME_MUTE)?;
    Ok(false)
}