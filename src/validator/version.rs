use crate::validator::Validate;
use anyhow::{bail, Result};
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
                    bail!("Invalid CAT version specified as part of CATV");
                }
                Ok(())
            }
            Some(_) => bail!("Invalid type value specified for CATV"),
            None => Ok(()),
        }
    }
}
