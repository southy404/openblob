use chrono::{Datelike, Local, Timelike};
use rand::Rng;
use serde_json::json;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::Emitter;

use crate::core::legacy::app_open_runtime;
use crate::core::legacy::browser_runtime;
use crate::core::legacy::input_runtime::*;
use crate::core::legacy::memory_runtime;
use crate::core::legacy::snip_runtime;
use crate::core::legacy::streaming_runtime;
use crate::core::legacy::system_media_runtime;
use crate::core::legacy::weather_runtime;
use crate::modules::command_router::CompanionAction;
use crate::modules::context::ActiveContext;
use crate::modules::i18n::replies::{reply, reply_with};

static ACTIVE_TIMER_ID: OnceLock<Mutex<u64>> = OnceLock::new();

fn active_timer_id_store() -> &'static Mutex<u64> {
    ACTIVE_TIMER_ID.get_or_init(|| Mutex::new(0))
}

fn next_timer_id() -> u64 {
    if let Ok(mut guard) = active_timer_id_store().lock() {
        *guard += 1;
        *guard
    } else {
        1
    }
}

fn current_timer_id() -> u64 {
    if let Ok(guard) = active_timer_id_store().lock() {
        *guard
    } else {
        0
    }
}

fn cancel_active_timer() {
    if let Ok(mut guard) = active_timer_id_store().lock() {
        *guard += 1;
    }
}

fn weekday_key(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "weekday_monday",
        chrono::Weekday::Tue => "weekday_tuesday",
        chrono::Weekday::Wed => "weekday_wednesday",
        chrono::Weekday::Thu => "weekday_thursday",
        chrono::Weekday::Fri => "weekday_friday",
        chrono::Weekday::Sat => "weekday_saturday",
        chrono::Weekday::Sun => "weekday_sunday",
    }
}

fn month_key(month: u32) -> &'static str {
    match month {
        1 => "month_january",
        2 => "month_february",
        3 => "month_march",
        4 => "month_april",
        5 => "month_may",
        6 => "month_june",
        7 => "month_july",
        8 => "month_august",
        9 => "month_september",
        10 => "month_october",
        11 => "month_november",
        12 => "month_december",
        _ => "month_january",
    }
}

fn localized_time_phrase(hour: u32, minute: u32) -> String {
    match minute {
        0 => reply_with("current_time_oclock", &[("hour", hour.to_string())]),
        15 => reply_with("current_time_quarter_past", &[("hour", hour.to_string())]),
        30 => reply_with(
            "current_time_half",
            &[("next_hour", ((hour + 1) % 24).to_string())],
        ),
        45 => reply_with(
            "current_time_quarter_to",
            &[("next_hour", ((hour + 1) % 24).to_string())],
        ),
        _ => reply_with(
            "current_time_exact",
            &[
                ("hour", format!("{hour:02}")),
                ("minute", format!("{minute:02}")),
            ],
        ),
    }
}

async fn maybe_speak_reply(reply_text: &str) {
    let lang = "de";

    if let Err(err) = crate::modules::tts::manager::speak(reply_text, Some(lang)).await {
        eprintln!("TTS error: {err}");
    }
}

fn success_reply(
    input: &str,
    context: &ActiveContext,
    reply_text: String,
) -> Result<String, String> {
    memory_runtime::register_successful_interaction(
        input,
        &context.app_name,
        &context.domain,
        &reply_text,
    );
    Ok(reply_text)
}

fn passthrough_reply(
    input: &str,
    context: &ActiveContext,
    reply_text: Result<String, String>,
) -> Result<String, String> {
    match reply_text {
        Ok(text) => success_reply(input, context, text),
        Err(err) => Err(err),
    }
}

pub async fn execute_legacy_voice_command(
    app: &tauri::AppHandle,
    input: &str,
    action: &CompanionAction,
    context: &ActiveContext,
) -> Result<String, String> {
    match action {
        CompanionAction::OpenApp {
            target,
            prefer_browser,
        } => passthrough_reply(
            input,
            context,
            app_open_runtime::open_app_target(target, *prefer_browser),
        ),

        CompanionAction::InsertText(text) => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            insert_text(text)?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::KeyPress(key) => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key(key)?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::KeyCombo(keys) => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key_combo(keys)?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::Save => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl('s')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::SaveAs => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl_shift('s')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::OpenFile => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl('o')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::NewFile => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl('n')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::Undo => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl('z')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::Redo => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            shortcut_ctrl('y')?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::Confirm => {
            let state = crate::modules::session_memory::get_state();

            if !state.last_suggested_url.is_empty() {
                app_open_runtime::open_url_prefer_browser(
                    &state.last_suggested_url,
                    false,
                    false,
                )?;
                return success_reply(
                    input,
                    context,
                    reply_with(
                        "confirm_opening_last_suggestion",
                        &[("title", state.last_suggested_title.clone())],
                    ),
                );
            }

            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("enter")?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::Clear => {
            let _target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("escape")?;
            success_reply(input, context, reply("ok"))
        }

        CompanionAction::NewTab => {
            passthrough_reply(input, context, browser_runtime::new_tab().await)
        }

        CompanionAction::Close => {
            passthrough_reply(input, context, browser_runtime::close_active_tab().await)
        }

        CompanionAction::CloseTab => {
            passthrough_reply(input, context, browser_runtime::close_active_tab().await)
        }

        CompanionAction::CloseTabByIndex { index } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_close_tab_by_index(*index).await,
            )
        }

        CompanionAction::NewWindow => {
            passthrough_reply(input, context, browser_runtime::new_window().await)
        }

        CompanionAction::BrowserBack => {
            passthrough_reply(input, context, browser_runtime::browser_back().await)
        }

        CompanionAction::BrowserForward => {
            passthrough_reply(input, context, browser_runtime::browser_forward().await)
        }

        CompanionAction::BrowserScrollDown => {
            passthrough_reply(input, context, browser_runtime::browser_scroll_down().await)
        }

        CompanionAction::BrowserScrollUp => {
            passthrough_reply(input, context, browser_runtime::browser_scroll_up().await)
        }

        CompanionAction::BrowserTypeText { text } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_type_text(text.clone()).await,
            )
        }

        CompanionAction::BrowserSubmit => {
            passthrough_reply(input, context, browser_runtime::browser_submit().await)
        }

        CompanionAction::BrowserClickBestMatch { text } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_click_best_match(text.clone()).await,
            )
        }

        CompanionAction::BrowserClickButtonByText { text } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_click_best_match(text.clone()).await,
            )
        }

        CompanionAction::BrowserContext => {
            let browser_ctx = browser_runtime::browser_get_context().await?;
            success_reply(
                input,
                context,
                reply_with(
                    "browser_context_summary",
                    &[
                        ("title", browser_ctx.title),
                        ("url", browser_ctx.url),
                        ("page_kind", browser_ctx.page_kind),
                        ("links", browser_ctx.visible_links.len().to_string()),
                        ("buttons", browser_ctx.visible_buttons.len().to_string()),
                        ("inputs", browser_ctx.visible_inputs.len().to_string()),
                    ],
                ),
            )
        }

        CompanionAction::GoogleSearch { query } => {
            passthrough_reply(input, context, browser_runtime::google_search(query).await)
        }

        CompanionAction::YouTubeSearch { query } => {
            passthrough_reply(input, context, browser_runtime::youtube_search(query).await)
        }

        CompanionAction::YouTubePlayTitle { title } => {
            passthrough_reply(input, context, browser_runtime::youtube_play_title(title).await)
        }

        CompanionAction::PlayOnService { service, query } => {
            passthrough_reply(
                input,
                context,
                app_open_runtime::play_on_service(service, query),
            )
        }

        CompanionAction::BrowserOpenUrl {
            url,
            new_tab,
            new_window,
            incognito,
        } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_open_url(
                    url.clone(),
                    *new_tab,
                    *new_window,
                    *incognito,
                )
                .await,
            )
        }

        CompanionAction::BrowserClickLinkByText { text, new_tab } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_click_link_by_text(text.clone(), *new_tab).await,
            )
        }

        CompanionAction::BrowserClickFirstResult => {
            passthrough_reply(input, context, browser_runtime::browser_click_first_result().await)
        }

        CompanionAction::BrowserClickNthResult { index } => {
            passthrough_reply(
                input,
                context,
                browser_runtime::browser_click_nth_result(*index).await,
            )
        }

        CompanionAction::VolumeUp => {
            passthrough_reply(input, context, system_media_runtime::volume_up())
        }

        CompanionAction::VolumeDown => {
            passthrough_reply(input, context, system_media_runtime::volume_down())
        }

        CompanionAction::SetVolume { percent } => {
            passthrough_reply(input, context, system_media_runtime::set_volume(*percent))
        }

        CompanionAction::Mute => {
            passthrough_reply(input, context, system_media_runtime::mute())
        }

        CompanionAction::Unmute => {
            passthrough_reply(input, context, system_media_runtime::unmute())
        }

        CompanionAction::ToggleMute => {
            passthrough_reply(input, context, system_media_runtime::toggle_mute())
        }

        CompanionAction::MediaPlayPause => {
            passthrough_reply(input, context, system_media_runtime::media_play_pause())
        }

        CompanionAction::MediaNext => {
            passthrough_reply(input, context, system_media_runtime::media_next())
        }

        CompanionAction::MediaPrev => {
            passthrough_reply(input, context, system_media_runtime::media_prev())
        }

        CompanionAction::YouTubePlay => {
            let target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("k")?;
            success_reply(
                input,
                context,
                reply_with("youtube_play_pause_triggered", &[("target", target)]),
            )
        }

        CompanionAction::YouTubePause => {
            let target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("k")?;
            success_reply(
                input,
                context,
                reply_with("youtube_play_pause_triggered", &[("target", target)]),
            )
        }

        CompanionAction::YouTubeNextVideo => {
            let target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key_combo(&["shift", "n"])?;
            success_reply(
                input,
                context,
                reply_with("youtube_next_video", &[("target", target)]),
            )
        }

        CompanionAction::YouTubeSeekForward => {
            let target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("l")?;
            success_reply(
                input,
                context,
                reply_with("youtube_seek_forward", &[("target", target)]),
            )
        }

        CompanionAction::YouTubeSeekBackward => {
            let target = app_open_runtime::ensure_external_focus("unknown")?;
            press_key("j")?;
            success_reply(
                input,
                context,
                reply_with("youtube_seek_backward", &[("target", target)]),
            )
        }

        CompanionAction::YouTubeSkipAd => {
            if browser_runtime::browser_click_best_match("skip ads".to_string())
                .await
                .is_ok()
            {
                return success_reply(input, context, reply("youtube_ad_skipped"));
            }

            if browser_runtime::browser_click_best_match("skip ad".to_string())
                .await
                .is_ok()
            {
                return success_reply(input, context, reply("youtube_ad_skipped"));
            }

            if browser_runtime::browser_click_best_match("überspringen".to_string())
                .await
                .is_ok()
            {
                return success_reply(input, context, reply("youtube_ad_skipped"));
            }

            if browser_runtime::browser_click_best_match("ueberspringen".to_string())
                .await
                .is_ok()
            {
                return success_reply(input, context, reply("youtube_ad_skipped"));
            }

            success_reply(input, context, reply("youtube_ad_skip_not_found"))
        }

        CompanionAction::WeatherToday { location } => {
            passthrough_reply(
                input,
                context,
                weather_runtime::weather_reply(location.clone()).await,
            )
        }

        CompanionAction::StreamOpenTitle {
            service,
            title,
            autoplay: _,
        } => {
            passthrough_reply(
                input,
                context,
                streaming_runtime::stream_open_title(service, title),
            )
        }

        CompanionAction::StreamRecommend {
            service,
            mood,
            genre,
            kind,
            trending,
        } => {
            passthrough_reply(
                input,
                context,
                streaming_runtime::stream_recommend(
                    service.clone(),
                    mood.clone(),
                    genre.clone(),
                    kind.clone(),
                    *trending,
                ),
            )
        }

        CompanionAction::StreamCapability { service } => {
            passthrough_reply(
                input,
                context,
                streaming_runtime::stream_capability(service.clone()),
            )
        }

        CompanionAction::StreamOpenLastSuggestion => {
            passthrough_reply(
                input,
                context,
                streaming_runtime::stream_open_last_suggestion(),
            )
        }

        CompanionAction::StreamMoreLikeLast => {
            passthrough_reply(
                input,
                context,
                streaming_runtime::stream_more_like_last(),
            )
        }

        CompanionAction::CoinFlip => {
            let key = if rand::thread_rng().gen_bool(0.5) {
                "coin_flip_heads"
            } else {
                "coin_flip_tails"
            };

            let reply_text = reply(key);
            maybe_speak_reply(&reply_text).await;
            success_reply(input, context, reply_text)
        }

        CompanionAction::RollDice => {
            let value = rand::thread_rng().gen_range(1..=6);
            success_reply(
                input,
                context,
                reply_with("roll_dice", &[("value", value.to_string())]),
            )
        }

        CompanionAction::CurrentTime => {
            let now = Local::now();
            success_reply(input, context, localized_time_phrase(now.hour(), now.minute()))
        }

        CompanionAction::CurrentDate => {
            let now = Local::now();

            success_reply(
                input,
                context,
                reply_with(
                    "current_date",
                    &[
                        ("weekday", reply(weekday_key(now.weekday()))),
                        ("day", now.day().to_string()),
                        ("month", reply(month_key(now.month()))),
                        ("year", now.year().to_string()),
                    ],
                ),
            )
        }

        CompanionAction::CancelTimer => {
            cancel_active_timer();

            let text = reply("timer_stopped");

            let _ = app.emit(
                "companion-timer-finished",
                json!({
                    "seconds": 0,
                    "text": text,
                }),
            );

            success_reply(input, context, reply("timer_stopped"))
        }

        CompanionAction::SetTimer { seconds } => {
            let seconds = (*seconds).max(1);
            let app_handle = app.clone();
            let timer_id = next_timer_id();

            let _ = app.emit(
                "companion-timer-started",
                json!({
                    "seconds": seconds,
                    "label": format!("{}:{:02} timer", seconds / 60, seconds % 60),
                    "startedAt": chrono::Utc::now().timestamp(),
                }),
            );

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(seconds)).await;

                if current_timer_id() != timer_id {
                    return;
                }

                let text = reply_with(
                    "timer_finished",
                    &[
                        ("minutes", (seconds / 60).to_string()),
                        ("seconds", (seconds % 60).to_string()),
                    ],
                );

                let _ = app_handle.emit(
                    "companion-timer-finished",
                    json!({
                        "seconds": seconds,
                        "text": text,
                    }),
                );

                let _ = crate::modules::tts::manager::speak(&text, Some("de")).await;
            });

            success_reply(
                input,
                context,
                reply_with(
                    "timer_started",
                    &[
                        ("minutes", (seconds / 60).to_string()),
                        ("seconds", format!("{:02}", seconds % 60)),
                    ],
                ),
            )
        }

        CompanionAction::TakeScreenshot => {
            let _ = app.emit("companion-snip-hotkey", ());
            success_reply(input, context, reply("snip_opened"))
        }

        CompanionAction::ExplainSelection => {
            Ok("NO_ACTION".into())
        }

        CompanionAction::None => Ok("NO_ACTION".into()),

        _ => success_reply(
            input,
            context,
            reply_with(
                "legacy_not_implemented",
                &[("action", format!("{:?}", action))],
            ),
        ),
    }
}
