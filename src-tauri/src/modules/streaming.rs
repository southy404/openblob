use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTitle {
    pub id: String,
    pub title: String,
    pub service: String,
    pub url: String,
    pub kind: String, // movie | series
    pub genres: Vec<String>,
    pub keywords: Vec<String>,
    pub year: Option<u16>,
    pub popularity: Option<f32>,
    pub trending_rank: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RecommendationQuery {
    pub service: Option<String>,
    pub mood: Option<String>,
    pub genre: Option<String>,
    pub kind: Option<String>, // movie | series
    pub trending: bool,
    pub exclude_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendationResult {
    pub title: StreamTitle,
    pub reason: String,
}

fn data_path(service: &str) -> PathBuf {
    let file_name = match normalize_service(service).as_str() {
        "netflix" => "netflix_titles.json",
        _ => "netflix_titles.json",
    };

    let candidates = [
        PathBuf::from(format!("src-tauri/data/{}", file_name)),
        PathBuf::from(format!("./src-tauri/data/{}", file_name)),
        PathBuf::from(format!("../src-tauri/data/{}", file_name)),
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("src-tauri")
            .join("data")
            .join(file_name),
    ];

    for path in candidates {
        if path.exists() {
            println!("[streaming] resolved data path: {}", path.display());
            return path;
        }
    }

    // fallback for debug visibility
    let fallback = PathBuf::from("src-tauri/data").join(file_name);
    println!(
        "[streaming] data path fallback used (file not found yet): {}",
        fallback.display()
    );
    fallback
}

fn normalize_service(service: &str) -> String {
    let s = normalize(service);

    match s.as_str() {
        "netflix" | "netflx" | "netfliks" | "netflxk" => "netflix".into(),
        _ => "netflix".into(),
    }
}

pub fn load_titles(service: &str) -> Result<Vec<StreamTitle>, String> {
    let path = data_path(service);
    println!("[streaming] loading titles from: {}", path.display());

    let text =
        fs::read_to_string(&path).map_err(|e| format!("Could not read streaming data: {e}"))?;

    match serde_json::from_str::<Vec<StreamTitle>>(&text) {
        Ok(parsed) => {
            println!("[streaming] loaded {} titles for {}", parsed.len(), service);
            Ok(parsed)
        }
        Err(e) => {
            println!("[streaming] JSON parse error: {}", e);
            Err(format!("Could not parse streaming data: {e}"))
        }
    }
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
        .replace('&', " and ")
        .replace(':', " ")
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn mood_aliases(mood: &str) -> Vec<&'static str> {
    match mood {
        "funny" | "lustig" | "witzig" | "comedy" => {
            vec!["funny", "light", "comedy", "humor", "witty", "entertaining"]
        }
        "dark" | "duester" | "dunkel" => {
            vec!["dark", "bleak", "intense", "disturbing", "brooding"]
        }
        "smart" | "klug" | "clever" => {
            vec!["smart", "clever", "mind bending", "complex", "thoughtful"]
        }
        "emotional" | "sad" | "traurig" => {
            vec!["emotional", "touching", "sad", "heartfelt", "moving"]
        }
        "action" => vec!["action", "explosive", "fast", "intense"],
        "scifi" | "sci fi" | "sci-fi" => vec!["sci-fi", "technology", "future", "space"],
        "thriller" => vec!["thriller", "suspense", "tense"],
        _ => vec![],
    }
}

fn canonical_mood(mood: Option<&str>) -> String {
    let m = mood.map(normalize).unwrap_or_default();

    if m.is_empty() {
        return String::new();
    }

    if contains_any(&m, &["lustig", "witzig", "funny", "comedy"]) {
        return "funny".into();
    }
    if contains_any(&m, &["dark", "duester", "dunkel", "bleak"]) {
        return "dark".into();
    }
    if contains_any(&m, &["smart", "clever", "klug", "mind", "complex"]) {
        return "smart".into();
    }
    if contains_any(&m, &["sad", "traurig", "emotional", "heartfelt"]) {
        return "emotional".into();
    }
    if contains_any(&m, &["action"]) {
        return "action".into();
    }
    if contains_any(&m, &["sci fi", "scifi", "sci-fi", "science fiction"]) {
        return "scifi".into();
    }
    if contains_any(&m, &["thriller", "suspense", "tense"]) {
        return "thriller".into();
    }

    m
}

fn canonical_genre(genre: Option<&str>) -> String {
    let g = genre.map(normalize).unwrap_or_default();

    if g.is_empty() {
        return String::new();
    }

    if contains_any(&g, &["comedy", "lustig"]) {
        return "comedy".into();
    }
    if contains_any(&g, &["animation", "animated"]) {
        return "animation".into();
    }
    if contains_any(&g, &["sci fi", "scifi", "sci-fi"]) {
        return "sci-fi".into();
    }
    if contains_any(&g, &["crime", "krimi"]) {
        return "crime".into();
    }
    if contains_any(&g, &["drama"]) {
        return "drama".into();
    }
    if contains_any(&g, &["action"]) {
        return "action".into();
    }
    if contains_any(&g, &["fantasy"]) {
        return "fantasy".into();
    }
    if contains_any(&g, &["thriller"]) {
        return "thriller".into();
    }
    if contains_any(&g, &["horror"]) {
        return "horror".into();
    }
    if contains_any(&g, &["documentary", "doku", "documentary"]) {
        return "documentary".into();
    }
    if contains_any(&g, &["mystery"]) {
        return "mystery".into();
    }

    g
}

fn canonical_kind(kind: Option<&str>) -> String {
    let k = kind.map(normalize).unwrap_or_default();

    if k.is_empty() {
        return String::new();
    }

    if contains_any(&k, &["movie", "film", "movie"]) {
        return "movie".into();
    }

    if contains_any(&k, &["series", "show", "serie", "tv show"]) {
        return "series".into();
    }

    k
}

fn title_tokens(title: &StreamTitle) -> Vec<String> {
    let mut out = Vec::new();

    out.push(normalize(&title.title));
    out.extend(title.genres.iter().map(|g| normalize(g)));
    out.extend(title.keywords.iter().map(|k| normalize(k)));
    out.push(normalize(&title.kind));

    out
}

pub fn find_title(service: &str, query: &str) -> Option<StreamTitle> {
    let titles = load_titles(service).ok()?;
    let q = normalize(query);

    if q.is_empty() {
        return None;
    }

    // 1) Hard exact match first
    for item in &titles {
        if normalize(&item.title) == q {
            return Some(item.clone());
        }
    }

    let q_words: Vec<&str> = q.split_whitespace().collect();

    let mut best: Option<StreamTitle> = None;
    let mut best_score = i32::MIN;

    for item in titles {
        let title_norm = normalize(&item.title);
        let title_words: Vec<&str> = title_norm.split_whitespace().collect();

        let mut score = 0i32;

        // Strong title matching
        if title_norm.contains(&q) {
            score += 900;
        } else if q.contains(&title_norm) {
            score += 760;
        }

        // Word overlap on title is very important
        let mut exact_word_hits = 0;
        let mut partial_word_hits = 0;

        for qw in &q_words {
            if qw.len() < 2 {
                continue;
            }

            if title_words.iter().any(|tw| tw == qw) {
                exact_word_hits += 1;
                score += 180;
            } else if title_words
                .iter()
                .any(|tw| tw.contains(qw) || qw.contains(tw))
            {
                partial_word_hits += 1;
                score += 90;
            }
        }

        // Bonus if all query words are covered by the title
        if !q_words.is_empty() && exact_word_hits == q_words.len() {
            score += 260;
        }

        // Small bonus if most words match
        if !q_words.is_empty() && exact_word_hits + partial_word_hits >= q_words.len() {
            score += 120;
        }

        // Keyword matching helps, but should not dominate title matches
        for kw in &item.keywords {
            let kw_norm = normalize(kw);

            if kw_norm == q {
                score += 120;
            } else if kw_norm.contains(&q) || q.contains(&kw_norm) {
                score += 50;
            }
        }

        // Genre matching is weak signal only
        for genre in &item.genres {
            let g = normalize(genre);
            if g == q {
                score += 25;
            } else if g.contains(&q) || q.contains(&g) {
                score += 10;
            }
        }

        // Popularity can slightly break ties
        if let Some(pop) = item.popularity {
            score += (pop * 4.0) as i32;
        }

        // Penalize extremely weak title matches
        if exact_word_hits == 0 && partial_word_hits == 0 && !title_norm.contains(&q) && !q.contains(&title_norm) {
            score -= 200;
        }

        if score > best_score {
            best_score = score;
            best = Some(item);
        }
    }

    if best_score >= 100 {
        best
    } else {
        None
    }
}

fn score_recommendation(item: &StreamTitle, q: &RecommendationQuery) -> (i32, String) {
    let mood = canonical_mood(q.mood.as_deref());
    let genre = canonical_genre(q.genre.as_deref());
    let kind = canonical_kind(q.kind.as_deref());

    let item_kind = normalize(&item.kind);
    let genres_norm: Vec<String> = item.genres.iter().map(|g| normalize(g)).collect();
    let keywords_norm: Vec<String> = item.keywords.iter().map(|k| normalize(k)).collect();

    let mut score = 0i32;
    let mut reasons: Vec<String> = Vec::new();

    if q.trending {
        if let Some(rank) = item.trending_rank {
            score += 500 - rank as i32;
            reasons.push("it's trending right now".into());
        } else {
            score -= 60;
        }
    }

    if !kind.is_empty() {
        if item_kind == kind {
            score += 220;
            reasons.push(format!("it's a {}", kind));
        } else {
            score -= 140;
        }
    }

    if !genre.is_empty() {
        if genres_norm.iter().any(|g| g == &genre || g.contains(&genre)) {
            score += 240;
            reasons.push(format!("it matches the {} vibe", genre));
        } else {
            score -= 80;
        }
    }

    if !mood.is_empty() {
        let aliases = mood_aliases(&mood);

        let mood_hit_keywords = keywords_norm
            .iter()
            .any(|kw| aliases.iter().any(|a| kw.contains(a)));

        let mood_hit_genres = genres_norm.iter().any(|g| match mood.as_str() {
            "funny" => g.contains("comedy") || g.contains("animation"),
            "dark" => g.contains("thriller") || g.contains("drama") || g.contains("sci-fi") || g.contains("crime"),
            "smart" => g.contains("sci-fi") || g.contains("mystery") || g.contains("crime") || g.contains("documentary"),
            "emotional" => g.contains("drama"),
            "action" => g.contains("action"),
            "scifi" => g.contains("sci-fi"),
            "thriller" => g.contains("thriller"),
            _ => false,
        });

        if mood_hit_keywords {
            score += 210;
        }

        if mood_hit_genres {
            score += 180;
        }

        if mood_hit_keywords || mood_hit_genres {
            reasons.push(match mood.as_str() {
                "funny" => "it fits a lighter, funny mood".into(),
                "dark" => "it matches a darker mood really well".into(),
                "smart" => "it feels like a smart pick".into(),
                "emotional" => "it fits a more emotional mood".into(),
                "action" => "it has the right action energy".into(),
                "scifi" => "it fits a sci-fi mood".into(),
                "thriller" => "it fits a thriller mood".into(),
                _ => "it matches your mood".into(),
            });
        }
    }

    if let Some(pop) = item.popularity {
        score += (pop * 12.0) as i32;
    }

    if let Some(year) = item.year {
        if year >= 2018 {
            score += 18;
        }
    }

    if q.exclude_titles
        .iter()
        .map(|t| normalize(t))
        .any(|t| t == normalize(&item.title))
    {
        score -= 10_000;
    }

    let reason = if reasons.is_empty() {
        "it seems like a strong fit".to_string()
    } else {
        reasons[0].clone()
    };

    (score, reason)
}

pub fn recommend_title(q: RecommendationQuery) -> Option<StreamTitle> {
    recommend_title_with_reason(q).map(|r| r.title)
}

pub fn recommend_title_with_reason(q: RecommendationQuery) -> Option<RecommendationResult> {
    let service = q.service.as_deref().unwrap_or("netflix");
    let titles = load_titles(service).ok()?;

    let mut scored: Vec<(i32, StreamTitle, String)> = Vec::new();

    for item in titles {
        let (score, reason) = score_recommendation(&item, &q);
        scored.push((score, item, reason));
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    scored.into_iter().next().and_then(|(score, item, reason)| {
        if score < 120 {
            None
        } else {
            Some(RecommendationResult {
                title: item,
                reason,
            })
        }
    })
}

pub fn recommend_alternatives(q: RecommendationQuery, limit: usize) -> Vec<RecommendationResult> {
    let service = q.service.as_deref().unwrap_or("netflix");
    let Ok(titles) = load_titles(service) else {
        return Vec::new();
    };

    let mut scored: Vec<(i32, StreamTitle, String)> = titles
        .into_iter()
        .map(|item| {
            let (score, reason) = score_recommendation(&item, &q);
            (score, item, reason)
        })
        .filter(|(score, _, _)| *score >= 120)
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, item, reason)| RecommendationResult {
            title: item,
            reason,
        })
        .collect()
}

pub fn trending_titles(service: &str, limit: usize) -> Vec<StreamTitle> {
    let Ok(mut titles) = load_titles(service) else {
        return Vec::new();
    };

    titles.sort_by(|a, b| {
        let ar = a.trending_rank.unwrap_or(u32::MAX);
        let br = b.trending_rank.unwrap_or(u32::MAX);

        if ar == br {
            let ap = a.popularity.unwrap_or(0.0);
            let bp = b.popularity.unwrap_or(0.0);
            bp.partial_cmp(&ap).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            ar.cmp(&br)
        }
    });

    titles
        .into_iter()
        .filter(|t| t.trending_rank.is_some())
        .take(limit)
        .collect()
}

pub fn parse_preference_query(input: &str, service_hint: Option<&str>) -> RecommendationQuery {
    let text = normalize(input);

    let service = if text.contains("netflix") {
        Some("netflix".to_string())
    } else {
        service_hint.map(|s| normalize_service(s))
    };

    let mood = if contains_any(&text, &["lustig", "funny", "comedy", "witzig"]) {
        Some("funny".into())
    } else if contains_any(&text, &["dark", "duester", "dunkel", "bleak"]) {
        Some("dark".into())
    } else if contains_any(&text, &["smart", "clever", "klug", "mind bending", "complex"]) {
        Some("smart".into())
    } else if contains_any(&text, &["sad", "emotional", "traurig", "heartfelt"]) {
        Some("emotional".into())
    } else if contains_any(&text, &["action"]) {
        Some("action".into())
    } else if contains_any(&text, &["sci fi", "scifi", "science fiction", "sci-fi"]) {
        Some("scifi".into())
    } else if contains_any(&text, &["thriller"]) {
        Some("thriller".into())
    } else {
        None
    };

    let genre = if contains_any(&text, &["comedy"]) {
        Some("comedy".into())
    } else if contains_any(&text, &["animation", "animated"]) {
        Some("animation".into())
    } else if contains_any(&text, &["crime", "krimi"]) {
        Some("crime".into())
    } else if contains_any(&text, &["drama"]) {
        Some("drama".into())
    } else if contains_any(&text, &["fantasy"]) {
        Some("fantasy".into())
    } else if contains_any(&text, &["documentary", "doku"]) {
        Some("documentary".into())
    } else if contains_any(&text, &["mystery"]) {
        Some("mystery".into())
    } else if contains_any(&text, &["action"]) {
        Some("action".into())
    } else if contains_any(&text, &["thriller"]) {
        Some("thriller".into())
    } else if contains_any(&text, &["sci fi", "scifi", "sci-fi"]) {
        Some("sci-fi".into())
    } else {
        None
    };

    let kind = if contains_any(&text, &["film", "movie"]) {
        Some("movie".into())
    } else if contains_any(&text, &["serie", "series", "show"]) {
        Some("series".into())
    } else {
        None
    };

    let trending = contains_any(
        &text,
        &["trend", "trending", "popular", "top", "hot", "gerade angesagt"],
    );

    RecommendationQuery {
        service,
        mood,
        genre,
        kind,
        trending,
        exclude_titles: Vec::new(),
    }
}

pub fn build_recommendation_reply(rec: &RecommendationResult) -> String {
    format!(
        "{} could be a great pick. {}. Want me to open it on {}?",
        rec.title.title, rec.reason, rec.title.service
    )
}

pub fn build_trending_reply(service: &str, limit: usize) -> String {
    let items = trending_titles(service, limit);

    if items.is_empty() {
        return format!("I couldn't find trending titles on {} right now.", service);
    }

    let names = items
        .iter()
        .map(|t| t.title.clone())
        .collect::<Vec<_>>()
        .join(", ");

    format!("Trending on {} right now: {}.", service, names)
}

pub fn find_title_or_recommend(service: &str, query: &str) -> Option<RecommendationResult> {
    if let Some(found) = find_title(service, query) {
        return Some(RecommendationResult {
            title: found,
            reason: "I found a direct title match".into(),
        });
    }

    let mut rq = parse_preference_query(query, Some(service));
    rq.service = Some(normalize_service(service));
    recommend_title_with_reason(rq)
}

pub fn best_followup_alternative(
    service: &str,
    previous_titles: &[String],
    original_query: Option<&str>,
) -> Option<RecommendationResult> {
    let mut rq = if let Some(q) = original_query {
        parse_preference_query(q, Some(service))
    } else {
        RecommendationQuery {
            service: Some(normalize_service(service)),
            mood: None,
            genre: None,
            kind: None,
            trending: false,
            exclude_titles: Vec::new(),
        }
    };

    rq.exclude_titles = previous_titles.to_vec();
    recommend_title_with_reason(rq)
}

pub fn is_streaming_service_name(input: &str) -> bool {
    matches!(
        normalize_service(input).as_str(),
        "netflix"
    )
}

pub fn looks_like_streaming_query(input: &str) -> bool {
    let text = normalize(input);

    contains_any(
        &text,
        &[
            "netflix",
            "stream",
            "watch",
            "play",
            "open",
            "movie",
            "film",
            "serie",
            "series",
            "trending",
            "recommend",
            "recommendation",
            "lustig",
            "funny",
            "dark",
            "smart"
        ],
    )
}

pub fn all_titles(service: &str) -> Vec<StreamTitle> {
    load_titles(service).unwrap_or_default()
}

pub fn search_titles(service: &str, query: &str, limit: usize) -> Vec<StreamTitle> {
    let Ok(titles) = load_titles(service) else {
        return Vec::new();
    };

    let q = normalize(query);
    let mut scored: Vec<(i32, StreamTitle)> = Vec::new();

    for item in titles {
        let tokens = title_tokens(&item);
        let mut score = 0;

        for token in &tokens {
            if token == &q {
                score += 300;
            } else if token.contains(&q) || q.contains(token) {
                score += 130;
            } else {
                score += q
                    .split_whitespace()
                    .filter(|w| token.contains(w))
                    .count() as i32
                    * 40;
            }
        }

        if let Some(pop) = item.popularity {
            score += (pop * 5.0) as i32;
        }

        if score >= 100 {
            scored.push((score, item));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(limit).map(|(_, item)| item).collect()
}