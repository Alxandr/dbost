use axum::body::Body;

pub trait HtmxHandler<T, S, B = Body>: Send + Sync + 'static {}
