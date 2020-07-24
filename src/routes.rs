use crate::{config::Config, db::Mod, errors::TryExt};
use semver::{Version, VersionReq};
use sqlx::SqlitePool;
use tokio::fs;
use warp::{Filter, Rejection, Reply};

pub fn handler(
    pool: &'static SqlitePool,
    config: &'static Config,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Send + Sync + Clone + 'static {
    let download = warp::path!("download" / String / Version)
        .and(warp::get())
        .and_then(move |id, ver| download(id, ver, config));

    let latest_matching = warp::path!("latest" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| latest_matching(id, req, pool));
    let all_matching = warp::path!("all" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| all_matching(id, req, pool));

    download.or(latest_matching).or(all_matching)
}

async fn download(id: String, ver: Version, config: &Config) -> Result<impl Reply, Rejection> {
    let contents = fs::read(
        config
            .downloads_path
            .join(id)
            .join(format!("{}/{}/{}", ver.major, ver.minor, ver.patch)),
    )
    .await
    .or_nf()?;
    Ok(contents)
}

async fn latest_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::latest_matching(&id, req, pool).await.or_ise()?;
    Ok(warp::reply::json(&m.or_nf()?))
}

async fn all_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::all_matching(&id, req, pool).await.or_ise()?;
    Ok(warp::reply::json(&m))
}
