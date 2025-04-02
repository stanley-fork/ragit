use warp::http::StatusCode;
use warp::reply::{Reply, with_status};

pub fn get_health() -> Box<dyn Reply> {
    Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    ))
}
