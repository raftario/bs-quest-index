mod config;
mod db;
mod errors;
mod routes;

use crate::config::Config;
use std::env;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::Filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::read(
        env::args()
            .nth(1)
            .unwrap_or_else(|| "config.json".to_owned()),
    )
    .await?;

    tracing_subscriber::fmt()
        .with_env_filter(
            config
                .log_level
                .as_ref()
                .map(AsRef::as_ref)
                .unwrap_or("info"),
        )
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let pool = db::connect(&config.database_url).await?;

    warp::serve(routes::handler(pool, config).with(warp::trace::request()))
        .run(([127, 0, 0, 1], config.port))
        .await;

    Ok(())
}

#[cfg(test)]
mod tests;
