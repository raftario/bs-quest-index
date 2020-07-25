use futures::{Stream, TryStreamExt};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqlitePool, SqliteQueryAs},
    FromRow,
};

#[tracing::instrument(level = "info")]
pub async fn connect(url: &str) -> sqlx::Result<&'static SqlitePool> {
    let pool = SqlitePool::new(&format!("sqlite://{}", url)).await?;
    sqlx::query(include_str!("../db.sql"))
        .execute(&pool)
        .await?;
    Ok(&*Box::leak(Box::new(pool)))
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
    pub async fn insert(id: &str, ver: &Version, pool: &SqlitePool) -> sqlx::Result<bool> {
        let affected =
            sqlx::query("INSERT OR IGNORE INTO mods (id, major, minor, patch) VALUES (?, ?, ?, ?)")
                .bind(id)
                .bind(ver.major as i64)
                .bind(ver.minor as i64)
                .bind(ver.patch as i64)
                .execute(pool)
                .await?;

        if affected == 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn resolve<'e>(
        id: &str,
        req: &'e VersionReq,
        pool: &'e SqlitePool,
    ) -> impl Stream<Item = sqlx::Result<Self>> + 'e {
        sqlx::query_as(
            "SELECT * FROM mods WHERE id = ? ORDER BY major DESC, minor DESC, patch DESC",
        )
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
