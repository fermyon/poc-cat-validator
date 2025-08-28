use anyhow::{Context, Result};
use common_access_token::{Token, VerificationOptions};

pub struct Cat<'a> {
    key: &'a str,
}
impl<'a> Cat<'a> {
    pub fn new(key: &'a str) -> Cat<'a> {
        Cat { key }
    }

    pub fn validate(&self, cat: &[u8]) -> Result<()> {
        let token = Token::from_bytes(cat)
            .with_context(|| "Invalid input provided could not decode CAT")?;

        token
            .verify(self.key.as_bytes())
            .with_context(|| "Failed to verfy signature")?;

        // Verify the claims
        let options = VerificationOptions::new()
            .verify_exp(true)
            .verify_nbf(true)
            .expected_issuer("example-issuer")
            .expected_audience("example-audience");

        token
            .verify_claims(&options)
            .expect("Failed to verify claims");
        Ok(())
    }
}
