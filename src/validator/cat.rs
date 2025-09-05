use anyhow::{Context, Error, Result};
use common_access_token::{Token, VerificationOptions};

use crate::{
    persistence::Persistence,
    validator::{kv::KvValidator, Validate},
};

pub struct CatValidationOptions {
    pub sync_validators: Vec<Box<dyn Validate>>,
    pub url: String,
    pub method: String,
    pub issuer: String,
    // refactor into kv sub struct
    pub country: Option<String>,
    pub client_ip: String,
    pub user_agent: Option<String>,

    pub validate_expiration: bool,
    pub validate_not_before: bool,
    pub audience: Option<String>,
    pub skip_kv_validations: bool,
}

pub struct Cat<'a> {
    key: &'a str,
}
impl<'a> Cat<'a> {
    pub fn new(key: &'a str) -> Cat<'a> {
        Cat { key }
    }

    pub async fn validate(&self, cat: &[u8], opts: CatValidationOptions) -> Result<()> {
        let token = Token::from_bytes(cat).with_context(|| "Token Decoding Failed")?;

        token
            .verify(self.key.as_bytes())
            .with_context(|| "Token Signature Validation Failed")?;

        if !opts.skip_kv_validations {
            let blocking_data = match Persistence::get_blocking_data() {
                Ok(data) => data,
                Err(e) => {
                    // we have to discuss how we should deal with this error
                    return Err(e);
                }
            };
            if blocking_data.any {
                let kv_validator = KvValidator::from(blocking_data);
                if kv_validator.is_subject_blocked(&token.claims.registered.sub, true) {
                    return Err(Error::msg("Subject blocked"));
                }
                if opts.country.is_some() && kv_validator.is_country_blocked(&opts.country.unwrap())
                {
                    return Err(Error::msg("Country or Region blocked"));
                }

                if opts.user_agent.is_some()
                    && kv_validator.is_user_agent_blocked(&opts.user_agent.unwrap())
                {
                    return Err(Error::msg("User Agent blocked"));
                }

                if kv_validator.is_ip_blocked(&opts.client_ip) {
                    return Err(Error::msg("IP address is blocked"));
                }
            }
        }

        let options = VerificationOptions::new()
            .verify_exp(opts.validate_expiration)
            .verify_nbf(opts.validate_not_before)
            .expected_issuer(opts.issuer.clone())
            .verify_catu(true)
            .require_aud(opts.audience.is_some())
            .expected_audience(opts.audience.unwrap_or_default())
            .uri(opts.url.clone())
            .verify_catm(true)
            .http_method(opts.method.clone().to_uppercase());
        token.verify_claims(&options)?;

        for v in opts.sync_validators {
            let claim_value = token.claims.custom.get(v.get_claim_key());
            if let Err(validation_error) = v.validate(claim_value) {
                return Err(validation_error);
            }
        }
        Ok(())
    }
}
