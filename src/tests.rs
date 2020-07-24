use semver::Version;
use std::path::PathBuf;
use tokio::fs;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::http::StatusCode;

#[tokio::test]
async fn test() {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let database_url = "target/test-database.db".to_owned();
    let downloads_path = PathBuf::from("target/test-downloads");

    fs::remove_file(&database_url).await.ok();
    fs::remove_dir_all(&downloads_path).await.ok();

    let config = Box::leak(Box::new(crate::config::Config {
        port: 0,
        database_url,
        downloads_path,
        log_level: None,
    }));
    let pool = crate::db::connect(&config.database_url).await.unwrap();

    let routes = crate::routes::handler(pool, config);

    // Upload some mods

    let reply = warp::test::request()
        .path("/mod/bshook/1.0.0")
        .method("POST")
        .body(b"bshook-1.0.0")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::CREATED);

    let reply = warp::test::request()
        .path("/mod/bshook/1.2.0")
        .method("POST")
        .body(b"bshook-1.2.0")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::CREATED);

    let reply = warp::test::request()
        .path("/mod/hsv/2.3.4")
        .method("POST")
        .body(b"hsv-2.3.4")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::CREATED);

    // Try uploading a duplicate

    let reply = warp::test::request()
        .path("/mod/bshook/1.0.0")
        .method("POST")
        .body(b"bshook-1.0.0 number two")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::CONFLICT);

    // Download a mod

    let reply = warp::test::request()
        .path("/mod/bshook/1.0.0")
        .method("GET")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(reply.body().as_ref(), b"bshook-1.0.0");

    // Try downloading a mod that doesn't exist

    let reply = warp::test::request()
        .path("/mod/bshook/3.0.0")
        .method("GET")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::NOT_FOUND);

    // Get the latest version of a mod matching the requirements

    let reply = warp::test::request()
        .path("/latest/bshook/^1")
        .method("GET")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<'_, crate::db::Mod>(reply.body().as_ref()).unwrap(),
        crate::db::Mod {
            id: "bshook".to_owned(),
            version: Version::new(1, 2, 0)
        }
    );

    // Get all the versions of a mod matching the requirements

    let reply = warp::test::request()
        .path("/all/bshook/^1")
        .method("GET")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<'_, Vec<crate::db::Mod>>(reply.body().as_ref()).unwrap(),
        vec![
            crate::db::Mod {
                id: "bshook".to_owned(),
                version: Version::new(1, 2, 0)
            },
            crate::db::Mod {
                id: "bshook".to_owned(),
                version: Version::new(1, 0, 0)
            }
        ]
    );

    // Try to get a version of a mod with requirements that can't match

    let reply = warp::test::request()
        .path("/latest/hsv/~3")
        .method("GET")
        .reply(&routes)
        .await;
    assert_eq!(reply.status(), StatusCode::NOT_FOUND);
}
