use crate::modules::i18n::replies::{reply, reply_with};
use crate::modules::session_memory;
use crate::modules::streaming;

use super::app_open_runtime::open_url_prefer_browser;

pub fn stream_open_title(service: &str, title: &str) -> Result<String, String> {
    if let Some(item) = streaming::find_title(service, title) {
        open_url_prefer_browser(&item.url, false, false)?;
        session_memory::set_last_suggestion(
            &item.title,
            service,
            &item.url,
            title,
        );

        Ok(reply_with(
            "stream_opening_title",
            &[
                ("title", item.title),
                ("service", service.to_string()),
            ],
        ))
    } else {
        Ok(reply_with(
            "stream_title_not_found",
            &[
                ("title", title.to_string()),
                ("service", service.to_string()),
            ],
        ))
    }
}

pub fn stream_recommend(
    service: Option<String>,
    mood: Option<String>,
    genre: Option<String>,
    kind: Option<String>,
    trending: bool,
) -> Result<String, String> {
    let query_text = format!(
        "{} {} {} {} {}",
        service.clone().unwrap_or_else(|| "netflix".into()),
        mood.clone().unwrap_or_default(),
        genre.clone().unwrap_or_default(),
        kind.clone().unwrap_or_default(),
        if trending { "trending" } else { "" }
    );

    let rec = streaming::recommend_title_with_reason(streaming::RecommendationQuery {
        service: service.clone(),
        mood,
        genre,
        kind,
        trending,
        exclude_titles: Vec::new(),
    });

    if let Some(rec) = rec {
        session_memory::set_last_suggestion(
            &rec.title.title,
            &rec.title.service,
            &rec.title.url,
            &query_text,
        );

        Ok(streaming::build_recommendation_reply(&rec))
    } else {
        Ok(reply("stream_no_recommendation"))
    }
}

pub fn stream_capability(service: Option<String>) -> Result<String, String> {
    let svc = service.unwrap_or_else(|| "netflix".into());

    Ok(reply_with(
        "stream_capability",
        &[("service", svc)],
    ))
}

pub fn stream_open_last_suggestion() -> Result<String, String> {
    let state = session_memory::get_state();

    if !state.last_suggested_url.is_empty() {
        open_url_prefer_browser(&state.last_suggested_url, false, false)?;
        return Ok(reply_with(
            "confirm_opening_last_suggestion",
            &[("title", state.last_suggested_title)],
        ));
    }

    Ok(reply("stream_no_recent_suggestion"))
}

pub fn stream_more_like_last() -> Result<String, String> {
    let state = session_memory::get_state();

    let rec = streaming::best_followup_alternative(
        if state.last_suggested_service.is_empty() {
            "netflix"
        } else {
            &state.last_suggested_service
        },
        &[state.last_suggested_title.clone()],
        if state.last_recommendation_query.is_empty() {
            None
        } else {
            Some(state.last_recommendation_query.as_str())
        },
    );

    if let Some(item) = rec {
        session_memory::set_last_suggestion(
            &item.title.title,
            &item.title.service,
            &item.title.url,
            &state.last_recommendation_query,
        );

        Ok(reply_with(
            "stream_more_like_last",
            &[
                ("title", item.title.title),
                ("reason", item.reason),
            ],
        ))
    } else {
        Ok(reply("stream_no_followup_option"))
    }
}