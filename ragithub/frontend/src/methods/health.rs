use super::{
    HandleError,
    RawResponse,
    get_backend,
    handler,
};
use warp::http::StatusCode;
use warp::reply::{Reply, with_status};

pub async fn get_health() -> Box<dyn Reply> {
    handler(get_health_().await)
}

async fn get_health_() -> RawResponse {
    let backend = get_backend();
    let request = reqwest::Client::new().get(&format!("{backend}/health"));
    let response = request.send().await.handle_error(500)?;

    if response.status() != 200 {
        return Err((response.status().as_u16(), String::from("backend is not healthy")));
    }

    Ok(Box::new(with_status(
        String::new(),
        StatusCode::from_u16(200).unwrap(),
    )))
}
