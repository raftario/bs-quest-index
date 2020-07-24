use std::fmt::Display;
use warp::reject::{Reject, Rejection};

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}

pub trait TryExt<T> {
    fn or_ise(self) -> Result<T, Rejection>;
}

impl<T, E: Display> TryExt<T> for Result<T, E> {
    fn or_ise(self) -> Result<T, Rejection> {
        self.map_err(|e| {
            tracing::error!("{}", e);
            warp::reject::custom(InternalServerError)
        })
    }
}
