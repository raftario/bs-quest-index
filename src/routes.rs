use crate::{db::Mod, errors::TryExt};
use semver::VersionReq;
use sqlx::SqlitePool;
use warp::{Filter, Rejection, Reply};

pub fn handler(
    pool: &'static SqlitePool,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Send + Sync + Clone + 'static {
    let latest_matching = warp::path!("latest" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| latest_matching(id, req, pool));
    let all_matching = warp::path!("all" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| all_matching(id, req, pool));
    (latest_matching).or(all_matching)
}

async fn latest_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::latest_matching(&id, req, pool).await.or_ise()?;
    match m {
        Some(m) => Ok(warp::reply::json(&m)),
        None => Err(warp::reject::not_found()),
    }
}

async fn all_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::all_matching(&id, req, pool).await.or_ise()?;
    Ok(warp::reply::json(&m))
}
