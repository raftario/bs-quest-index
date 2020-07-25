use crate::{config::Config, db::Mod, errors::TryExt};
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use semver::{Version, VersionReq};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::fs;
use warp::{
    http::{HeaderValue, StatusCode},
    Filter, Rejection, Reply,
};

#[inline]
fn one() -> usize {
    1
}

#[derive(Debug, Deserialize)]
struct ResolveQuery {
    #[serde(default = "VersionReq::any")]
    req: VersionReq,
    #[serde(default = "one")]
    limit: usize,
}

pub fn handler(
    pool: &'static SqlitePool,
    config: &'static Config,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Send + Sync + Clone + 'static {
    let resolve = warp::path!(String)
        .and(warp::get())
        .and(warp::query())
        .and_then(move |id, query| resolve(id, query, pool));

    let download = warp::path!(String / Version)
        .and(warp::get())
        .and_then(move |id, ver| download(id, ver, config));
    let upload = warp::path!(String / Version)
        .and(warp::post())
        .and(auth(config))
        .and(warp::body::bytes())
        .and_then(move |id, ver, contents| upload(id, ver, contents, pool, config));

    resolve
        .or(download)
        .or(upload)
        .recover(crate::errors::handle_rejection)
}

fn auth(
    config: &'static Config,
) -> impl Filter<Extract = (), Error = Rejection> + Send + Sync + Clone + 'static {
    warp::header::optional("Authorization")
        .and_then(move |k: Option<HeaderValue>| async move {
            let k = match k {
                Some(k) => k,
                None => return Err(warp::reject::custom(crate::errors::Unauthorized)),
            };

            if config.keys.contains(k.to_str().or_ise()?) {
                Ok(())
            } else {
                Err(warp::reject::custom(crate::errors::Unauthorized))
            }
        })
        .untuple_one()
}

#[tracing::instrument(level = "debug", skip(pool))]
async fn resolve(
    id: String,
    query: ResolveQuery,
    pool: &SqlitePool,
) -> Result<impl Reply, Rejection> {
    let mut mods = Mod::resolve(&id, &query.req, pool);

    match query.limit {
        // 1 => last version, found or not found
        1 => Ok(warp::reply::json(&mods.next().await.or_nf()?.or_ise()?)),
        // 0 => all versions
        0 => Ok(warp::reply::json(
            &mods.try_collect::<Vec<_>>().await.or_ise()?,
        )),
        // n => n latest versions
        n => Ok(warp::reply::json(
            &mods.take(n).try_collect::<Vec<_>>().await.or_ise()?,
        )),
    }
}

#[tracing::instrument(level = "debug", skip(config))]
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

#[tracing::instrument(level = "debug", skip(pool, config))]
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
