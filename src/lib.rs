use anyhow::{Context, Result};
use common_access_token::{current_timestamp, Algorithm, KeyId, RegisteredClaims, TokenBuilder};
use serde_json::json;
use spin_sdk::http::{IntoResponse, Params, Request, Response, ResponseBuilder, Router};
use spin_sdk::http_component;

use crate::validator::Cat;
mod validator;

const KEY: &'static str = "my-super-fancy-and-secret-key";

#[http_component]
fn handle_cat_validator(req: Request) -> anyhow::Result<impl IntoResponse> {
    let mut router = Router::default();
    router.get("/foo", get_resource_handler);
    router.post("/create", generate_test_token);
    Ok(router.handle(req))
}

fn get_resource_handler(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Some(token) = req
        .header("Authorization")
        .and_then(|v| v.as_str())
        .and_then(|v| v.strip_prefix("bearer "))
    else {
        return Ok(Response::new(401, ()));
    };
    let decoded_token = base64_url::decode(token).unwrap();
    if Cat::new(KEY).validate(decoded_token.as_slice()).is_err() {
        return Ok(Response::new(403, ()));
    }
    Ok(Response::new(200, "You made it"))
}

fn generate_test_token(_: Request, _: Params) -> Result<impl IntoResponse> {
    let now = current_timestamp();
    let token = TokenBuilder::new()
        .algorithm(Algorithm::HmacSha256)
        .protected_key_id(KeyId::string("my-key-id"))
        .registered_claims(
            RegisteredClaims::new()
                .with_issuer("example-issuer")
                .with_subject("example-subject")
                .with_audience("example-audience")
                .with_expiration(now + 3600) // 1 hour from now
                .with_not_before(now)
                .with_issued_at(now)
                .with_cti(b"token-id-1234".to_vec()),
        )
        .sign(KEY.as_bytes())
        .with_context(|| "Failed to sign token")?;
    let token_bytes = token.to_bytes().with_context(|| "Failed to encode token")?;
    let token_str = base64_url::encode(&token_bytes);
    let payload = serde_json::to_string_pretty(&json!({"token": token_str}))
        .with_context(|| "Failed to serialize response payload")?;

    Ok(ResponseBuilder::new(200)
        .header("content-type", "application/json")
        .body(payload)
        .build())
}
