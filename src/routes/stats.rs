use std::str::from_utf8;

use askama::Template;
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use super::AppError;

/// A user that is authorized to access the stats endpoint.
///
/// No fields are required, we just need to know that the user is authorized. In
/// a production application you would probably want to have some kind of user
/// ID or similar here.
pub struct User;

#[derive(sqlx::FromRow, Deserialize, Debug, Clone)]
pub struct City {
    name: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = axum::http::Response<axum::body::Body>;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|header| header.to_str().ok());

        if let Some(auth_header) = auth_header {
            if auth_header.starts_with("Basic ") {
                let credentials = auth_header.trim_start_matches("Basic ");
                let decoded = base64::decode(credentials).unwrap_or_default();
                let credential_str = from_utf8(&decoded).unwrap_or("");

                // Our username and password are hardcoded here.
                // In a real app, you'd want to read them from the environment.
                if credential_str == "forecast:forecast" {
                    return Ok(User);
                }
            }
        }

        let reject_response = axum::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(
                "WWW-Authenticate",
                "Basic realm=\"Please enter your credentials\"",
            )
            .body(axum::body::Body::from("Unauthorized"))
            .unwrap();

        Err(reject_response)
    }
}

#[derive(Template)]
#[template(path = "stats.html")]
pub struct StatsTemplate {
    pub cities: Vec<City>,
}

async fn get_last_cities(pool: &PgPool) -> Result<Vec<City>, AppError> {
    let cities = sqlx::query_as::<_, City>("SELECT name FROM cities ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await?;
    Ok(cities)
}

pub async fn stats(_user: User, State(pool): State<PgPool>) -> Result<StatsTemplate, AppError> {
    let cities = get_last_cities(&pool).await?;
    Ok(StatsTemplate { cities })
}
