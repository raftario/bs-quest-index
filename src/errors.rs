use std::fmt::Display;
use warp::reject::{Reject, Rejection};

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}

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
            warp::reject::not_found()
        })
    }
}

impl<T> TryExt<T> for Option<T> {
    fn or_ise(self) -> Result<T, Rejection> {
        self.ok_or_else(|| warp::reject::custom(InternalServerError))
    }

    fn or_nf(self) -> Result<T, Rejection> {
        self.ok_or_else(warp::reject::not_found)
    }
}
