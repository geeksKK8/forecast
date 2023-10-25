use std::net::SocketAddr;

use anyhow::Context;
use axum::{routing::get, Router};
use forecast::routes;
use sqlx::{Executor, PgPool};

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
    pool.execute(include_str!("../init.sql"))
        .await
        .context("Failed to initialize database")?;
    // let db_connection_str = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    // let pool = sqlx::PgPool::connect(&db_connection_str)
    //     .await
    //     .context("can't connect to database")?;
    let app = Router::new()
        .route("/", get(routes::index))
        .route("/weather", get(routes::weather))
        .route("/stats", get(routes::stats))
        .with_state(pool);
    // let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    // axum::Server::bind(&addr)
    //     .serve(app.into_make_service())
    //     .await?;
    Ok(app.into())
}
