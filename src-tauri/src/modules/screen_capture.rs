use chrono::Local;
use screenshots::Screen;
use std::fs;
use std::path::{Path, PathBuf};

fn ensure_snip_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("openblob-snips");

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Could not create snips dir: {e}"))?;
    }

    Ok(dir)
}

fn pick_screen_for_point(x: i32, y: i32) -> Result<Screen, String> {
    let screens = Screen::all().map_err(|e| format!("Could not enumerate screens: {e}"))?;

    for screen in screens {
        let info = screen.display_info;
        let sx = info.x;
        let sy = info.y;
        let sw = info.width as i32;
        let sh = info.height as i32;

        if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
            return Ok(screen);
        }
    }

    Err("No screen found for selected region.".into())
}

pub fn capture_region_to_file(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    if width == 0 || height == 0 {
        return Err("Invalid snip region.".into());
    }

    let screen = pick_screen_for_point(x, y)?;

    let info = screen.display_info;
    let local_x = x - info.x;
    let local_y = y - info.y;

    if local_x < 0 || local_y < 0 {
        return Err("Selected region is outside screen bounds.".into());
    }

    let image = screen
        .capture_area(local_x, local_y, width, height)
        .map_err(|e| format!("Could not capture screen area: {e}"))?;

    let dir = ensure_snip_dir()?;
    let filename = format!("snip_{}.png", Local::now().format("%Y%m%d_%H%M%S_%3f"));
    let path = dir.join(filename);

    image
        .save(&path)
        .map_err(|e| format!("Could not save snip image: {e}"))?;

    Ok(path.to_string_lossy().to_string())
}

pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}