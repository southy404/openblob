use crate::modules::i18n::replies::{reply, reply_with};
use crate::modules::system;

pub fn volume_up() -> Result<String, String> {
    system::change_system_volume(0.08)?;
    Ok(reply("system_volume_up"))
}

pub fn volume_down() -> Result<String, String> {
    system::change_system_volume(-0.08)?;
    Ok(reply("system_volume_down"))
}

pub fn set_volume(percent: u8) -> Result<String, String> {
    Err(reply_with(
        "system_volume_set_not_implemented",
        &[("percent", percent.to_string())],
    ))
}

pub fn mute() -> Result<String, String> {
    system::set_system_mute(true)?;
    Ok(reply("system_mute_on"))
}

pub fn unmute() -> Result<String, String> {
    system::set_system_mute(false)?;
    Ok(reply("system_mute_off"))
}

pub fn toggle_mute() -> Result<String, String> {
    system::toggle_system_mute()?;
    Ok(reply("system_mute_toggle"))
}

pub fn media_play_pause() -> Result<String, String> {
    system::media_play_pause()?;
    Ok(reply("system_media_play_pause"))
}

pub fn media_next() -> Result<String, String> {
    system::media_next_track()?;
    Ok(reply("system_media_next"))
}

pub fn media_prev() -> Result<String, String> {
    system::media_prev_track()?;
    Ok(reply("system_media_prev"))
}