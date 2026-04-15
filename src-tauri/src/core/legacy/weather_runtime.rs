use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::modules::i18n::replies::{reply, reply_with};

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodingResponse {
    results: Option<Vec<OpenMeteoGeocodingResult>>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodingResult {
    name: String,
    country: Option<String>,
    admin1: Option<String>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoForecastResponse {
    current: Option<OpenMeteoCurrent>,
    daily: Option<OpenMeteoDaily>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f32,
    apparent_temperature: Option<f32>,
    weather_code: Option<i32>,
    wind_speed_10m: Option<f32>,
    is_day: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoDaily {
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    precipitation_probability_max: Option<Vec<i32>>,
    weather_code: Option<Vec<i32>>,
}

fn weather_code_label(code: i32) -> String {
    match code {
        0 => reply("weather_code_clear"),
        1 | 2 | 3 => reply("weather_code_partly_cloudy"),
        45 | 48 => reply("weather_code_fog"),
        51 | 53 | 55 => reply("weather_code_drizzle"),
        56 | 57 => reply("weather_code_freezing_drizzle"),
        61 | 63 | 65 => reply("weather_code_rain"),
        66 | 67 => reply("weather_code_freezing_rain"),
        71 | 73 | 75 | 77 => reply("weather_code_snow"),
        80 | 81 | 82 => reply("weather_code_showers"),
        85 | 86 => reply("weather_code_snow_showers"),
        95 => reply("weather_code_thunderstorm"),
        96 | 99 => reply("weather_code_thunderstorm_hail"),
        _ => reply("weather_code_changeable"),
    }
}

fn build_clothing_advice(temp_now: f32, temp_max: f32, rain_prob: i32, wind_kmh: f32) -> String {
    let mut items: Vec<String> = Vec::new();

    if temp_now <= 5.0 || temp_max <= 8.0 {
        items.push(reply("weather_item_warm_jacket"));
    } else if temp_now <= 12.0 || temp_max <= 15.0 {
        items.push(reply("weather_item_light_jacket"));
    } else if temp_max >= 24.0 {
        items.push(reply("weather_item_light_clothes"));
    } else {
        items.push(reply("weather_item_normal_transition"));
    }

    if rain_prob >= 55 {
        items.push(reply("weather_item_umbrella"));
    }

    if wind_kmh >= 30.0 {
        items.push(reply("weather_item_windproof"));
    }

    match items.len() {
        0 => reply("weather_advice_none"),
        1 => reply_with("weather_advice_single", &[("item1", items[0].clone())]),
        2 => reply_with(
            "weather_advice_double",
            &[
                ("item1", items[0].clone()),
                ("item2", items[1].clone()),
            ],
        ),
        _ => {
            let last = items.pop().unwrap_or_default();
            reply_with(
                "weather_advice_multi",
                &[
                    ("items", items.join(", ")),
                    ("last", last),
                ],
            )
        }
    }
}

pub async fn weather_reply(location: Option<String>) -> Result<String, String> {
    let place = location
        .unwrap_or_else(|| "Berlin".to_string())
        .trim()
        .to_string();

    if place.is_empty() {
        return Err(reply("weather_location_not_provided"));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Weather client konnte nicht erstellt werden: {e}"))?;

    let geo_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=de&format=json",
        urlencoding::encode(&place)
    );

    let geo = client
        .get(&geo_url)
        .send()
        .await
        .map_err(|e| format!("Geocoding fehlgeschlagen: {e}"))?;

    if !geo.status().is_success() {
        let status = geo.status();
        let text = geo.text().await.unwrap_or_default();
        return Err(format!("Geocoding Fehler {}: {}", status, text));
    }

    let geo_data: OpenMeteoGeocodingResponse = geo
        .json()
        .await
        .map_err(|e| format!("Geocoding-Antwort konnte nicht gelesen werden: {e}"))?;

    let location_hit = geo_data
        .results
        .and_then(|mut results| results.drain(..).next())
        .ok_or_else(|| {
            reply_with(
                "weather_location_not_found",
                &[("place", place.clone())],
            )
        })?;

    let forecast_url = format!(
        concat!(
            "https://api.open-meteo.com/v1/forecast",
            "?latitude={}",
            "&longitude={}",
            "&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,is_day",
            "&daily=temperature_2m_max,temperature_2m_min,precipitation_probability_max,weather_code",
            "&timezone=auto",
            "&forecast_days=1"
        ),
        location_hit.latitude, location_hit.longitude
    );

    let forecast = client
        .get(&forecast_url)
        .send()
        .await
        .map_err(|e| format!("Wetterabfrage fehlgeschlagen: {e}"))?;

    if !forecast.status().is_success() {
        let status = forecast.status();
        let text = forecast.text().await.unwrap_or_default();
        return Err(format!("Wetter API Fehler {}: {}", status, text));
    }

    let forecast_data: OpenMeteoForecastResponse = forecast
        .json()
        .await
        .map_err(|e| format!("Wetterdaten konnten nicht gelesen werden: {e}"))?;

    let current = forecast_data
        .current
        .ok_or_else(|| reply("weather_no_current_data"))?;

    let daily = forecast_data
        .daily
        .ok_or_else(|| reply("weather_no_daily_data"))?;

    let temp_now = current.temperature_2m;
    let feels_like = current.apparent_temperature.unwrap_or(temp_now);
    let wind = current.wind_speed_10m.unwrap_or(0.0);

    let temp_max = daily.temperature_2m_max.first().copied().unwrap_or(temp_now);
    let temp_min = daily.temperature_2m_min.first().copied().unwrap_or(temp_now);
    let rain_prob = daily
        .precipitation_probability_max
        .as_ref()
        .and_then(|v: &Vec<i32>| v.first().copied())
        .unwrap_or(0);

    let weather_code = current
        .weather_code
        .or_else(|| daily.weather_code.as_ref().and_then(|v: &Vec<i32>| v.first().copied()))
        .unwrap_or(-1);

    let summary = weather_code_label(weather_code);
    let advice = build_clothing_advice(temp_now, temp_max, rain_prob, wind);

    let pretty_place = match (&location_hit.admin1, &location_hit.country) {
        (Some(admin1), Some(country)) if !admin1.is_empty() => {
            format!("{}, {}, {}", location_hit.name, admin1, country)
        }
        (_, Some(country)) => format!("{}, {}", location_hit.name, country),
        _ => location_hit.name.clone(),
    };

    Ok(reply_with(
        "weather_summary",
        &[
            ("place", pretty_place),
            ("temp_now", format!("{temp_now:.0}")),
            ("feels_like", format!("{feels_like:.0}")),
            ("summary", summary),
            ("temp_min", format!("{temp_min:.0}")),
            ("temp_max", format!("{temp_max:.0}")),
            ("rain_prob", rain_prob.to_string()),
            ("wind", format!("{wind:.0}")),
            ("advice", advice),
        ],
    ))
}