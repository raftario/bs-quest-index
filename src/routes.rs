use crate::{config::Config, db::Mod, errors::TryExt};
use bytes::Bytes;
use semver::{Version, VersionReq};
use sqlx::SqlitePool;
use tokio::fs;
use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn handler(
    pool: &'static SqlitePool,
    config: &'static Config,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Send + Sync + Clone + 'static {
    let download = warp::path!("download" / String / Version)
        .and(warp::get())
        .and_then(move |id, ver| download(id, ver, config));
    let upload = warp::path!("upload" / String / Version)
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(move |id, ver, contents| upload(id, ver, contents, pool, config));

    let latest_matching = warp::path!("latest" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| latest_matching(id, req, pool));
    let all_matching = warp::path!("all" / String / VersionReq)
        .and(warp::get())
        .and_then(move |id, req| all_matching(id, req, pool));

    (download).or(upload).or(latest_matching).or(all_matching)
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

async fn upload(
    id: String,
    ver: Version,
    contents: Bytes,
    pool: &SqlitePool,
    config: &Config,
) -> Result<impl Reply, Rejection> {
    if !Mod::insert(&id, &ver, pool).await.or_ise()? {
        return Ok(warp::reply::with_status("", StatusCode::CONFLICT));
    }

    let dir = config
        .downloads_path
        .join(id)
        .join(format!("{}/{}", ver.major, ver.minor));
    fs::create_dir_all(&dir).await.or_ise()?;

    let file = dir.join(ver.patch.to_string());
    fs::write(file, contents).await.or_ise()?;

    Ok(warp::reply::with_status("", StatusCode::CREATED))
}

async fn latest_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::latest_matching(&id, &req, pool).await.or_ise()?;
    Ok(warp::reply::json(&m.or_nf()?))
}

async fn all_matching(
    id: String,
    req: VersionReq,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let m = Mod::all_matching(&id, &req, pool).await.or_ise()?;
    Ok(warp::reply::json(&m))
}
