use anyhow::Error;
use common_access_token::{cat_keys, CborValue};

use crate::validator::{Convert, Validate};

pub struct CatCountryValidator {
    pub country: String,
}

impl Validate for CatCountryValidator {
    fn get_claim_key(&self) -> &i32 {
        &cat_keys::CATGEOISO3166
    }

    fn validate(&self, claim: Option<&common_access_token::CborValue>) -> anyhow::Result<()> {
        let allowed_countries = match claim {
            None => return Ok(()), // no country claim present
            Some(CborValue::Array(c)) => {
                if c.is_empty() {
                    return Ok(()); // no countries explicitly mentioned in defined country claim
                }
                c
            }
            _ => {
                return Err(Error::msg(
                    "Invalid format for CATGEOISO3166 was expecting an Array",
                ))
            }
        };
        match allowed_countries
            .iter()
            .filter_map(|val| val.as_string())
            .map(|country| country.to_uppercase().trim().to_string())
            .collect::<Vec<String>>()
            .contains(&self.country.to_uppercase().trim().to_string())
        {
            true => Ok(()), // country in granted by claim
            false => Err(Error::msg(
                "Request origin not granted by claim (CATGEOISO3166)",
            )),
        }
    }
}
