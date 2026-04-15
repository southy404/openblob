use enigo::{Direction, Enigo, Key, Keyboard, Settings};

fn send_keys<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut Enigo) -> Result<(), String>,
{
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;
    f(&mut enigo)?;
    Ok(())
}

pub fn insert_text(text: &str) -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn press_key(key: &str) -> Result<(), String> {
    send_keys(|enigo| {
        enigo
            .key(parse_key(key), Direction::Click)
            .map_err(|e| e.to_string())
    })
}

pub fn press_key_combo(keys: &[&str]) -> Result<(), String> {
    send_keys(|enigo| {
        for key in keys {
            enigo
                .key(parse_key(key), Direction::Press)
                .map_err(|e| e.to_string())?;
        }

        for key in keys.iter().rev() {
            enigo
                .key(parse_key(key), Direction::Release)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    })
}

pub fn shortcut_ctrl(key: char) -> Result<(), String> {
    send_keys(|enigo| {
        enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Unicode(key), Direction::Click).map_err(|e| e.to_string())?;
        enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())
    })
}

pub fn shortcut_ctrl_shift(key: char) -> Result<(), String> {
    send_keys(|enigo| {
        enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Shift, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Unicode(key), Direction::Click).map_err(|e| e.to_string())?;
        enigo.key(Key::Shift, Direction::Release).map_err(|e| e.to_string())?;
        enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())
    })
}

pub fn trigger_copy_shortcut() -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn parse_key(key: &str) -> Key {
    match key {
        "ctrl" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "enter" => Key::Return,
        "escape" => Key::Escape,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "j" => Key::Unicode('j'),
        "k" => Key::Unicode('k'),
        "l" => Key::Unicode('l'),
        "n" => Key::Unicode('n'),
        "o" => Key::Unicode('o'),
        "r" => Key::Unicode('r'),
        "s" => Key::Unicode('s'),
        "t" => Key::Unicode('t'),
        "w" => Key::Unicode('w'),
        "y" => Key::Unicode('y'),
        "z" => Key::Unicode('z'),
        _ => Key::Unicode(' '),
    }
}