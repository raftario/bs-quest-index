use futures::{Stream, StreamExt, TryStreamExt};
use semver::{Version, VersionReq};
use serde::Serialize;
use sqlx::{
    sqlite::{SqlitePool, SqliteQueryAs},
    FromRow,
};

#[tracing::instrument(level = "info")]
pub async fn connect(url: &str) -> sqlx::Result<&'static SqlitePool> {
    let pool = SqlitePool::new(url).await?;
    sqlx::query(include_str!("../db.sql"))
        .execute(&pool)
        .await?;
    Ok(&*Box::leak(Box::new(pool)))
}

#[derive(Serialize)]
pub struct Mod {
    pub id: String,
    pub version: Version,
}

#[derive(FromRow)]
struct DbMod {
    id: String,

    major: i64,
    minor: i64,
    patch: i64,
}

impl From<DbMod> for Mod {
    fn from(db_mod: DbMod) -> Self {
        Self {
            id: db_mod.id,
            version: Version::new(
                db_mod.major as u64,
                db_mod.minor as u64,
                db_mod.patch as u64,
            ),
        }
    }
}

impl Mod {
    #[tracing::instrument(level = "debug", skip(pool))]
    pub async fn latest_matching(
        id: &str,
        req: VersionReq,
        pool: &SqlitePool,
    ) -> sqlx::Result<Option<Self>> {
        Self::matching(id, req, pool).next().await.transpose()
    }

    #[tracing::instrument(level = "debug", skip(pool))]
    pub async fn all_matching(
        id: &str,
        req: VersionReq,
        pool: &SqlitePool,
    ) -> sqlx::Result<Vec<Self>> {
        Self::matching(id, req, pool).try_collect().await
    }

    fn matching<'e>(
        id: &str,
        req: VersionReq,
        pool: &'e SqlitePool,
    ) -> impl Stream<Item = sqlx::Result<Self>> + 'e {
        sqlx::query_as("SELECT * FROM mods WHERE id = ? ORDER BY major, minor, patch DESC")
            .bind(id)
            .fetch(pool)
            .try_filter_map(move |m: DbMod| {
                futures::future::ready({
                    let m = Self::from(m);
                    if req.matches(&m.version) {
                        sqlx::Result::Ok(Some(m))
                    } else {
                        sqlx::Result::Ok(None)
                    }
                })
            })
    }
}
