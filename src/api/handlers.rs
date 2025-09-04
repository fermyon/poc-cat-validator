use std::collections::BTreeMap;

use anyhow::{Context, Result};
use common_access_token::{
    cat_keys, catm, catu, catv, current_timestamp, uri_components, Algorithm, CborValue, KeyId,
    RegisteredClaims, TokenBuilder,
};
use garde::Validate;
use serde_json::json;
use spin_sdk::http::{IntoResponse, Params, Request, Response, ResponseBuilder};

use crate::{
    api::models::{GenerateTokenRequestModel, ItemsModel, ValidateTokenRequestModel},
    persistence::{BlockedClaimType, Persistence},
    validator::{Cat, CatValidationOptions},
};
// this could be wizened
const KEY: &str = "my-super-fancy-and-secret-key";

pub fn get_blocking_data(_: Request, _: Params) -> Result<impl IntoResponse> {
    let data = Persistence::get_blocking_data()?;
    Ok(ResponseBuilder::new(200)
        .header("content-type", "application/json")
        .body(data)
        .build())
}

pub fn remove_items_from_blocklist(req: Request, p: Params) -> Result<impl IntoResponse> {
    let Some(kind) = p.get("kind") else {
        return Ok(Response::new(400, "Bad Request"));
    };
    let Ok(kind) = BlockedClaimType::try_from(kind) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    let Ok(model) = serde_json::from_slice::<ItemsModel<String>>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    match Persistence::remove_items_from_blocklist(kind, model.values) {
        Ok(_) => Ok(Response::new(200, ())),
        Err(_) => Ok(Response::new(500, ())),
    }
}

pub fn add_items_to_blocklist(req: Request, p: Params) -> Result<impl IntoResponse> {
    let Some(kind) = p.get("kind") else {
        return Ok(Response::new(400, "Bad Request"));
    };
    let Ok(kind) = BlockedClaimType::try_from(kind) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    let Ok(model) = serde_json::from_slice::<ItemsModel<String>>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    match Persistence::add_items_to_blocklist(kind, model.values) {
        Ok(_) => Ok(Response::new(200, ())),
        Err(_) => Ok(Response::new(500, ())),
    }
}

pub fn remove_asns_from_blocklist(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Ok(model) = serde_json::from_slice::<ItemsModel<u32>>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    match Persistence::remove_asns_from_blocklist(model.values) {
        Ok(_) => Ok(Response::new(200, ())),
        Err(_) => Ok(Response::new(500, ())),
    }
}

pub async fn add_asns_to_blocklist(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Ok(model) = serde_json::from_slice::<ItemsModel<u32>>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    match Persistence::add_asns_to_blocklist(model.values).await {
        Ok(_) => Ok(Response::new(200, ())),
        Err(_) => Ok(Response::new(500, ())),
    }
}

pub async fn validate_token_simple(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Ok(model) = serde_json::from_slice::<ValidateTokenRequestModel>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };
    let Ok(decoded_token) = base64_url::decode(model.token.as_str()) else {
        return Ok(Response::new(
            400,
            "Bad Request (could not decode Common Access Token",
        ));
    };

    match Cat::new(KEY)
        .validate(&decoded_token, model.into_non_kv_validation_options())
        .await
    {
        Ok(_) => Ok(Response::new(200, ())),
        Err(e) => Ok(Response::new(403, format!("{}", e))),
    }
}

pub async fn validate_token(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Ok(model) = serde_json::from_slice::<ValidateTokenRequestModel>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    let Ok(decoded_token) = base64_url::decode(model.token.as_str()) else {
        return Ok(Response::new(
            400,
            "Bad Request (could not decode Common Access Token)",
        ));
    };

    match Cat::new(KEY)
        .validate(&decoded_token, CatValidationOptions::from(model))
        .await
    {
        Ok(_) => Ok(Response::new(200, ())),
        Err(e) => Ok(Response::new(403, format!("{}", e))),
    }
}

pub fn generate_test_token(req: Request, _: Params) -> Result<impl IntoResponse> {
    let Ok(model) = serde_json::from_slice::<GenerateTokenRequestModel>(req.body()) else {
        return Ok(Response::new(400, "Bad Request"));
    };

    if let Err(e) = model.validate() {
        return Ok(Response::new(400, format!("Bad Request ({})", e)));
    };

    let now = current_timestamp();

    let mut catu_components = BTreeMap::new();
    catu_components.insert(uri_components::SCHEME, catu::exact_match("https"));
    catu_components.insert(uri_components::HOST, catu::exact_match("my-streaming.api"));
    catu_components.insert(uri_components::PATH, catu::prefix_match("/media"));

    catu_components.insert(uri_components::EXTENSION, catu::exact_match(".mp4"));

    let allowed_methods = vec!["GET"];
    let token = TokenBuilder::new()
        .algorithm(Algorithm::HmacSha256)
        .protected_key_id(KeyId::string("my-key-id"))
        .registered_claims(
            RegisteredClaims::new()
                .with_issuer(model.issuer.clone())
                .with_subject(model.subject)
                .with_audience(model.audience)
                .with_expiration(now + (model.expiration_in_hours * 60 * 60))
                .with_not_before(now)
                .with_issued_at(now)
                .with_cti(model.token_identifier.as_bytes()),
        )
        .custom_cbor(cat_keys::CATV, catv::create())
        .custom_cbor(cat_keys::CATU, catu::create(catu_components))
        .custom_array(cat_keys::CATM, catm::create(allowed_methods))
        .custom_array(
            cat_keys::CATGEOISO3166,
            catm::create(model.countries.iter().map(|c| c.as_str()).collect()),
        )
        .custom_map(cat_keys::CATH, {
            let mut ua_map = BTreeMap::new();
            ua_map.insert(1, CborValue::Integer(3));
            ua_map.insert(2, CborValue::Text("Mozilla".to_string()));
            let mut x_map = BTreeMap::new();
            x_map.insert(1, CborValue::Integer(0));
            x_map.insert(2, CborValue::Text("Lorem".to_string()));

            let mut map = std::collections::BTreeMap::new();
            map.insert(1, CborValue::Text("User-Agent".to_string()));
            map.insert(2, CborValue::Map(ua_map));
            map.insert(3, CborValue::Text("X-FWF-Custom-Header".to_string()));
            map.insert(4, CborValue::Map(x_map));
            map
        })
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
