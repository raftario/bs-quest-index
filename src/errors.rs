use std::fmt::Display;
use warp::{
    http::StatusCode,
    reject::{Reject, Rejection},
    Reply,
};

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}

#[derive(Debug)]
pub struct NotFound;
impl Reject for NotFound {}

#[derive(Debug)]
pub struct Unauthorized;
impl Reject for Unauthorized {}

pub trait TryExt<T> {
    fn or_ise(self) -> Result<T, Rejection>;
    fn or_nf(self) -> Result<T, Rejection>;
}

impl<T, E: Display> TryExt<T> for Result<T, E> {
    fn or_ise(self) -> Result<T, Rejection> {
        self.map_err(|e| {
            tracing::error!("{}", e);
            warp::reject::custom(InternalServerError)
        })
    }

    fn or_nf(self) -> Result<T, Rejection> {
        self.map_err(|e| {
            tracing::info!("{}", e);
            warp::reject::custom(NotFound)
        })
    }
}

impl<T> TryExt<T> for Option<T> {
    fn or_ise(self) -> Result<T, Rejection> {
        self.ok_or_else(|| warp::reject::custom(InternalServerError))
    }

    fn or_nf(self) -> Result<T, Rejection> {
        self.ok_or_else(|| warp::reject::custom(NotFound))
    }
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() || err.find::<NotFound>().is_some() {
        Ok(warp::reply::with_status("Not Found", StatusCode::NOT_FOUND))
    } else if err.find::<InternalServerError>().is_some() {
        Ok(warp::reply::with_status(
            "Internal Server Error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if err.find::<Unauthorized>().is_some() {
        Ok(warp::reply::with_status(
            "Unauthorized",
            StatusCode::UNAUTHORIZED,
        ))
    } else {
        Err(err)
    }
}
