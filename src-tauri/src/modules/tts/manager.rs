use super::kokoro;
use super::piper;
use super::tts_config::{detect_lang_from_text, preferred_lang, TtsConfig, TtsProvider};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TTS_GENERATION: AtomicU64 = AtomicU64::new(0);

pub async fn speak(text: &str, lang: Option<&str>) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let generation = TTS_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = piper::stop_current();

    let config = TtsConfig::default();

    if !config.enabled {
        return Ok(());
    }

    let resolved_lang = resolve_lang(trimmed, lang, &config);

    let result = match resolved_lang.as_str() {
        "de" => speak_with_provider(trimmed, &config.de_provider, &config.de_voice, &config).await,
        _ => speak_with_provider(trimmed, &config.en_provider, &config.en_voice, &config).await,
    };

    if generation != TTS_GENERATION.load(Ordering::SeqCst) {
        return Ok(());
    }

    result
}

pub async fn stop() -> Result<(), String> {
    TTS_GENERATION.fetch_add(1, Ordering::SeqCst);
    piper::stop_current()?;
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

#[cfg(test)]
mod tests {
    use super::stop;

    #[test]
    fn stop_tts_is_idempotent() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime");

        runtime.block_on(async {
            stop().await.expect("first stop should succeed");
            stop().await.expect("second stop should succeed");
        });
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
            let text = text.to_string();
            let fallback_text = text.clone();
            let piper_exe = config.piper_exe.clone();
            let piper_models_dir = config.piper_models_dir.clone();
            let voice = trimmed_voice.to_string();

            let result = tokio::task::spawn_blocking(move || {
                piper::speak(&text, &piper_exe, &piper_models_dir, &voice)
            })
            .await
            .map_err(|e| format!("Piper-Task fehlgeschlagen: {e}"))?;

            #[cfg(target_os = "macos")]
            {
                if result.is_err() {
                    // Fallback to macOS built-in TTS if Piper isn't available.
                    Command::new("say")
                        .arg(fallback_text)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .map_err(|e| format!("TTS failed: {e}"))?;
                    return Ok(());
                }
            }

            result
        }
        TtsProvider::Kokoro => kokoro::speak(text, &config.kokoro_base_url, trimmed_voice).await,
    }
}
