use anyhow::{bail, Context, Result};
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
        let token = Token::from_bytes(cat)
            .with_context(|| "Invalid input provided could not decode CAT")?;

        token
            .verify(self.key.as_bytes())
            .with_context(|| "Failed to verfy signature")?;
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

        if !opts.skip_kv_validations {
            let blocking_data = match Persistence::get_blocking_data() {
                Ok(data) => data,
                Err(e) => {
                    // we have to discuss how we should deal with this error
                    return Err(e);
                }
            };
            let kv_validator = KvValidator::from(blocking_data);
            if kv_validator.is_subject_blocked(&token.claims.registered.sub, true) {
                bail!("Subject blocked");
            }
            let country = String::from("de");
            let client_ip = String::from("127.0.0.1");
            let user_agent = String::from("fake");

            if kv_validator.is_country_blocked(&country) {
                bail!("Country or Region blocked");
            }

            if kv_validator.is_ip_blocked_by_asn(&client_ip) {
                bail!("IP blocked by ASN")
            }

            if kv_validator.is_ip_blocked(&client_ip) {
                bail!("IP locked")
            }

            if kv_validator.is_user_agent_blocked(&user_agent) {
                bail!("User Agent blocked")
            }
        }

        for v in opts.sync_validators {
            let claim_value = token.claims.custom.get(v.get_claim_key());
            match v.validate(claim_value) {
                Err(validation_error) => {
                    println!("{}", validation_error);
                    return Err(validation_error);
                }
                Ok(_) => (),
            }
        }
        Ok(())
    }
}
