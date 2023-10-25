use anyhow::Context;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use super::AppError;

#[derive(Deserialize, Debug, Clone)]
pub struct GeoResponse {
    pub results: Vec<LatLong>,
}

#[derive(sqlx::FromRow, Deserialize, Debug, Clone)]
pub struct LatLong {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
pub struct WeatherQuery {
    pub city: String,
}

#[derive(Deserialize, Debug)]
pub struct WeatherResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub hourly: Hourly,
}

#[derive(Deserialize, Debug)]
pub struct Hourly {
    pub time: Vec<String>,
    pub temperature_2m: Vec<f64>,
}

#[derive(Template, Deserialize, Debug)]
#[template(path = "weather.html")]
pub struct WeatherDisplay {
    pub city: String,
    pub forecasts: Vec<Forecast>,
}

#[derive(Deserialize, Debug)]
pub struct Forecast {
    pub date: String,
    pub temperature: String,
}

impl WeatherDisplay {
    fn new(city: String, response: WeatherResponse) -> Self {
        let display = WeatherDisplay {
            city,
            forecasts: response
                .hourly
                .time
                .iter()
                .zip(response.hourly.temperature_2m.iter())
                .map(|(date, temp)| Forecast {
                    date: date.to_string(),
                    temperature: temp.to_string(),
                })
                .collect(),
        };
        display
    }
}

async fn get_lat_long(pool: &PgPool, name: &str) -> Result<LatLong, anyhow::Error> {
    let lat_long = sqlx::query_as::<_, LatLong>(
        "SELECT lat AS latitude, long AS longitude FROM cities WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    if let Some(lat_long) = lat_long {
        return Ok(lat_long);
    }

    let lat_long = fetch_lat_long(name).await?;
    sqlx::query("INSERT INTO cities (name, lat, long) VALUES ($1, $2, $3)")
        .bind(name)
        .bind(lat_long.latitude)
        .bind(lat_long.longitude)
        .execute(pool)
        .await?;

    Ok(lat_long)
}

async fn fetch_lat_long(city: &str) -> Result<LatLong, anyhow::Error> {
    let endpoint = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        city
    );
    let response = reqwest::get(&endpoint).await?.json::<GeoResponse>().await?;
    response.results.get(0).cloned().context("No results found")
}

async fn fetch_weather(lat_long: LatLong) -> Result<WeatherResponse, anyhow::Error> {
    let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m",
        lat_long.latitude, lat_long.longitude
    );
    let response = reqwest::get(&endpoint)
        .await?
        .json::<WeatherResponse>()
        .await?;
    Ok(response)
}

pub async fn weather(
    Query(params): Query<WeatherQuery>,
    State(pool): State<PgPool>,
) -> Result<WeatherDisplay, AppError> {
    let lat_long = get_lat_long(&pool, &params.city).await?;
    let weather = fetch_weather(lat_long).await?;
    Ok(WeatherDisplay::new(params.city, weather))
}
