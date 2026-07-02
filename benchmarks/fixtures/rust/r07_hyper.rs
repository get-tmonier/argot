use hyper::{Body, Request, Response};
pub async fn handle(_req: Request<Body>) -> Response<Body> { Response::new(Body::from("ok")) }
