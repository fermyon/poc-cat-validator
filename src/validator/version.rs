use crate::validator::Validate;
use anyhow::{Error, Result};
use common_access_token::{cat_keys, CborValue};

pub struct CatVersionValidator {}

impl Validate for CatVersionValidator {
    fn get_claim_key(&self) -> &i32 {
        &cat_keys::CATV
    }

    fn validate(&self, claim: Option<&CborValue>) -> Result<()> {
        match claim {
            Some(CborValue::Integer(v)) => {
                if v != &1 {
                    return Err(Error::msg("Invalid CAT version specified as part of CATV"));
                }
                Ok(())
            }
            Some(_) => Err(Error::msg("Invalid type value specified for CATV")),
            None => Ok(()),
        }
    }
}
