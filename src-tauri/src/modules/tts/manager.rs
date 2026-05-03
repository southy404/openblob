use super::kokoro;
use super::piper;
use super::tts_config::{detect_lang_from_text, preferred_lang, TtsConfig, TtsProvider};
use std::process::{Command, Stdio};

pub async fn speak(text: &str, lang: Option<&str>) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let config = TtsConfig::default();

    if !config.enabled {
        return Ok(());
    }

    let resolved_lang = resolve_lang(trimmed, lang, &config);

    match resolved_lang.as_str() {
        "de" => {
            speak_with_provider(
                trimmed,
                &config.de_provider,
                &config.de_voice,
                &config,
            )
            .await
        }
        _ => {
            speak_with_provider(
                trimmed,
                &config.en_provider,
                &config.en_voice,
                &config,
            )
            .await
        }
    }
}

pub async fn stop() -> Result<(), String> {
    Ok(())
}

fn resolve_lang(text: &str, lang: Option<&str>, config: &TtsConfig) -> String {
    if let Some(explicit) = lang {
        let normalized = explicit.trim().to_lowercase();
        if !normalized.is_empty() {
            return normalize_lang_code(&normalized);
        }
    }

    let detected = detect_lang_from_text(text);
    if !detected.trim().is_empty() {
        return normalize_lang_code(&detected);
    }

    let preferred = preferred_lang();
    if !preferred.trim().is_empty() {
        return normalize_lang_code(&preferred);
    }

    normalize_lang_code(&config.default_lang)
}

fn normalize_lang_code(lang: &str) -> String {
    let lower = lang.trim().to_lowercase();

    match lower.as_str() {
        "de" | "de-de" | "deutsch" | "german" => "de".to_string(),
        "en" | "en-us" | "en-gb" | "english" => "en".to_string(),
        _ if lower.starts_with("de") => "de".to_string(),
        _ => "en".to_string(),
    }
}

async fn speak_with_provider(
    text: &str,
    provider: &TtsProvider,
    voice: &str,
    config: &TtsConfig,
) -> Result<(), String> {
    let trimmed_voice = voice.trim();
    if trimmed_voice.is_empty() {
        return Err("Keine TTS-Stimme konfiguriert.".to_string());
    }

    match provider {
        TtsProvider::Piper => {
            let result = piper::speak(
                text,
                &config.piper_exe,
                &config.piper_models_dir,
                trimmed_voice,
            );

            #[cfg(target_os = "macos")]
            {
                if result.is_err() {
                    // Fallback to macOS built-in TTS if Piper isn't available.
                    Command::new("say")
                        .arg(text)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .map_err(|e| format!("TTS failed: {e}"))?;
                    return Ok(());
                }
            }

            result
        }
        TtsProvider::Kokoro => {
            kokoro::speak(
                text,
                &config.kokoro_base_url,
                trimmed_voice,
            )
            .await
        }
    }
}
