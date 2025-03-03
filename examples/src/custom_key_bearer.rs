use http::{StatusCode, request::Request};
use axum::{routing::get, Router, error_handling::HandleErrorLayer, BoxError};
use tower_governor::{
    governor::{GovernorConfigBuilder},
    key_extractor::KeyExtractor,
    errors::{GovernorError, display_error},
    GovernorLayer,
};
use tower::ServiceBuilder;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
struct UserToken;

impl KeyExtractor for UserToken {
    type Key = String;
    type KeyExtractionError = GovernorError;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, Self::KeyExtractionError> {
        req.headers()
            .get("Authorization")
            .and_then(|token| token.to_str().ok())
            .and_then(|token| token.strip_prefix("Bearer "))
            .and_then(|token| Some(token.trim().to_owned()))
            .ok_or(
                GovernorError::Other{
                    code: StatusCode::UNAUTHORIZED,
                    msg: Some("You don't have permission to access".to_string()),
                    headers: None
                }
            )
    }
    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(format!("{}", key))
    }
    fn name(&self) -> &'static str { 
        "UserToken"
     }
}

async fn hello() -> &'static str {
    "Hello world"
}

#[tokio::main]
async fn main() {

    // Configure tracing if desired
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Allow bursts with up to five requests per IP address
    // and replenishes one element every two seconds
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(20)
        .burst_size(5)
        .key_extractor(UserToken)
        .use_headers()
        .finish()
        .unwrap();
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(hello))
        .layer(
            ServiceBuilder::new()
                // this middleware goes above `GovernorLayer` because it will receive
                // errors returned by `GovernorLayer`
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
                .layer(GovernorLayer {
                    config: &governor_conf,
                })
        );
        

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
