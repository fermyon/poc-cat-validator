use std::collections::HashMap;

use garde::Validate;
use serde::Deserialize;

use crate::validator::{
    CatCountryValidator, CatHeaderValidator, CatNipValidator, CatValidationOptions,
    CatVersionValidator,
};

#[derive(Deserialize)]
pub struct ItemsModel<T> {
    pub values: Vec<T>,
}

#[derive(Deserialize, Validate)]
pub struct GenerateTokenRequestModel {
    #[garde(skip)]
    pub issuer: String,
    #[garde(skip)]
    pub subject: String,
    #[garde(skip)]
    pub audience: String,
    // ISO 3166 conform coutry or region codes
    #[garde(length(min = 1))]
    pub countries: Vec<String>,
    #[garde(range(min = 1))]
    pub expiration_in_hours: u64,
    #[garde(skip)]
    pub token_identifier: String,
}

#[derive(Deserialize, Validate)]
pub struct ValidateTokenRequestModel {
    #[garde(skip)]
    pub token: String,
    #[garde(url)]
    pub url: String,
    #[garde(skip)]
    pub method: String,
    #[garde(skip)]
    pub issuer: String,
    #[garde(length(min = 1))]
    pub headers: HashMap<String, String>,
    #[garde(skip)]
    #[serde(default = "validate_by_default")]
    pub validate_not_before: Option<bool>,
    #[garde(skip)]
    #[serde(default = "validate_by_default")]
    pub validate_expiration: Option<bool>,
    #[garde(skip)]
    pub audience: Option<String>,
    #[garde(ip)]
    pub client_ip: String,
    #[garde(skip)]
    pub country: Option<String>,
}

impl ValidateTokenRequestModel {
    pub fn into_non_kv_validation_options(self) -> CatValidationOptions {
        let user_agent = self
            .headers
            .iter()
            .find(|k| k.0.eq_ignore_ascii_case("User-Agent"))
            .and_then(|found| Some(found.1.clone()));
        let mut opts = CatValidationOptions {
            url: self.url.clone(),
            validate_expiration: self.validate_expiration.unwrap_or(true),
            validate_not_before: self.validate_not_before.unwrap_or(true),
            audience: self.audience.clone(),
            skip_kv_validations: true,
            country: None,
            method: self.method.clone(),
            issuer: self.issuer.clone(),
            client_ip: self.client_ip.clone(),
            user_agent,
            sync_validators: vec![
                Box::new(CatVersionValidator {}),
                Box::new(CatHeaderValidator {
                    headers: self.headers,
                }),
                Box::new(CatNipValidator {
                    client_ip: self.client_ip.clone(),
                }),
            ],
        };
        if self.country.is_some() {
            opts.sync_validators.push(Box::new(CatCountryValidator {
                country: self.country.unwrap(),
            }))
        }
        opts
    }
}

impl From<ValidateTokenRequestModel> for CatValidationOptions {
    fn from(value: ValidateTokenRequestModel) -> Self {
        let user_agent = value
            .headers
            .iter()
            .find(|k| k.0.eq_ignore_ascii_case("User-Agent"))
            .and_then(|found| Some(found.1.clone()));
        let mut opts = Self {
            url: value.url.clone(),
            validate_expiration: value.validate_expiration.unwrap_or(true),
            validate_not_before: value.validate_not_before.unwrap_or(true),
            skip_kv_validations: false,
            country: value.country.clone(),
            audience: value.audience.clone(),
            method: value.method.clone(),
            client_ip: value.client_ip.clone(),
            user_agent,
            issuer: value.issuer.clone(),
            sync_validators: vec![
                Box::new(CatVersionValidator {}),
                Box::new(CatHeaderValidator {
                    headers: value.headers,
                }),
                Box::new(CatNipValidator {
                    client_ip: value.client_ip,
                }),
            ],
        };

        if value.country.is_some() {
            opts.sync_validators.push(Box::new(CatCountryValidator {
                country: value.country.unwrap(),
            }))
        }
        opts
    }
}

fn validate_by_default() -> Option<bool> {
    Some(true)
}
