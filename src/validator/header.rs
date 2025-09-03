use anyhow::{bail, Error, Result};
use std::collections::HashMap;

use common_access_token::{cat_keys, CborValue};

use crate::validator::{Convert, Validate};

pub struct CatHeaderValidator {
    pub headers: HashMap<String, String>,
}

impl Validate for CatHeaderValidator {
    fn get_claim_key(&self) -> &i32 {
        &cat_keys::CATH
    }

    fn validate(&self, claim: Option<&CborValue>) -> Result<()> {
        let map = match claim {
            None => {
                // CATH claim not present
                return Ok(());
            }
            Some(CborValue::Map(map)) => map,
            _ => return Err(Error::msg("Invalid format provided for CATH")),
        };
        if map.len() % 2 != 0 {
            return Err(Error::msg("cath claim has unexpected length"));
        }
        let count = map.len() as i32;
        let mut i = 1_i32;
        let mut j = 2_i32;

        println!("Map has {} length trying to load at {} and {}", count, i, j);
        map.keys().for_each(|k| println!("Key: {}", k));
        while j + 1 <= count {
            let header_name = map
                .get(&i)
                .expect(format!("Map has no entry at {}", i).as_str())
                .as_string()
                .expect(format!("Could not turn value at {} into string", i).as_str());
            let header_value = map.get(&j).and_then(|hv| hv.as_match_kind());
            match header_value {
                None => {
                    if !self.headers.contains_key(&header_name) {
                        return Err(Error::msg(format!(
                            "Required HTTP Header {header_name} not presented"
                        )));
                    }
                }
                Some(mk) => {
                    let value = self.headers.get(&header_name);
                    if value.is_none() {
                        return Err(Error::msg(format!(
                            "Required HTTP Header {header_name} not presented"
                        )));
                    }
                    if mk.validate(value.unwrap().clone()).is_err() {
                        return Err(Error::msg(format!(
                            "Presented HTTP Header {header_name} has invalid value"
                        )));
                    }
                }
            }

            i += 2;
            j += 2;
        }

        Ok(())
    }
}
