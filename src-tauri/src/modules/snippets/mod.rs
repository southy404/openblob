use crate::modules::profile::user_profile::load_or_create_user_profile;

pub fn has_snippet(key: &str) -> bool {
    get_snippet(key).is_some()
}

pub fn get_snippet(key: &str) -> Option<String> {
    let profile = load_or_create_user_profile().ok()?;

    match key.trim().to_lowercase().as_str() {
        "email" => profile.email_address.clone(),
        "github" => profile.github_url.clone(),
        "discord" => profile.discord_url.clone(),
        "signature" => profile.signature.clone(),
        _ => None,
    }
}