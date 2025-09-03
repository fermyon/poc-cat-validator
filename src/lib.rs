use spin_sdk::http::{IntoResponse, Request, Router};
use spin_sdk::http_component;

use crate::api::handlers::{
    add_asns_to_blocklist, add_items_to_blocklist, generate_test_token, get_blocking_data,
    remove_asns_from_blocklist, remove_items_from_blocklist, validate_token, validate_token_simple,
};

mod api;
mod asn_resolver;
mod persistence;
mod validator;

#[http_component]
fn handle_cat_validator(req: Request) -> anyhow::Result<impl IntoResponse> {
    let mut router = Router::default();
    router.post_async("/validate/simple", validate_token_simple);
    router.post_async("/validate", validate_token);
    router.post("/api/tests/tokens", generate_test_token);
    router.post("/api/blocking-data/simple/:kind", add_items_to_blocklist);
    router.delete(
        "/api/blocking-data/simple/:kind",
        remove_items_from_blocklist,
    );

    router.post_async("/api/blocking-data/asns", add_asns_to_blocklist);
    router.delete("/api/blocking-data/asns", remove_asns_from_blocklist);

    router.get("/api/blocking-data", get_blocking_data);
    Ok(router.handle(req))
}
