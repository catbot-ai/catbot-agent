use anyhow::{Result, *};
use regex::Regex;
use serde::de::DeserializeOwned;
use std::result::Result::Ok;

pub fn try_parse_json_with_trailing_comma_removal<T: DeserializeOwned>(
    json_string: &str,
) -> Result<T> {
    match serde_json::from_str(json_string) {
        Ok(parsed) => Ok(parsed),
        Err(original_error) => {
            // TODO: refactor
            let cleaned_json_string = fix_trailing_commas(json_string);
            serde_json::from_str(&cleaned_json_string).map_err(|e| {
                anyhow!(
                    "Failed to parse cleaned JSON: {}. Original error: {}\njson_string: {}",
                    e,
                    original_error,
                    json_string
                )
            })
        }
    }
}

fn fix_trailing_commas(json_str: &str) -> String {
    // Regex pattern to match a comma followed by optional whitespace and a closing bracket/brace
    let re = Regex::new(r#",(\s*[\]}])"#).unwrap();

    // Replace ",]" or ",}" (with optional whitespace) with just "]" or "}"
    re.replace_all(json_str, "$1").to_string()
}
