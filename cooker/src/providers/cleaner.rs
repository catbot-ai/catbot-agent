use anyhow::{Result, *};
use serde::de::DeserializeOwned;
use std::result::Result::Ok;

pub fn try_parse_json_with_trailing_comma_removal<T: DeserializeOwned>(
    json_string: &str,
) -> Result<T> {
    match serde_json::from_str(json_string) {
        Ok(parsed) => Ok(parsed),
        Err(original_error) => {
            // Attempt to remove trailing commas and try parsing again
            let cleaned_json_string = remove_trailing_commas(json_string);
            match serde_json::from_str(&cleaned_json_string) {
                Ok(parsed) => Ok(parsed),
                Err(_) => Err(anyhow!(original_error)), // Return the original error if cleaning fails
            }
        }
    }
}

pub fn remove_trailing_commas(json_string: &str) -> String {
    let mut cleaned_string = String::with_capacity(json_string.len());
    let mut in_string = false;
    let mut last_char = ' ';

    for c in json_string.chars() {
        match c {
            '"' => {
                in_string = !in_string;
                cleaned_string.push(c);
            }
            ',' if !in_string && (last_char == '}' || last_char == ']') => {
                // Ignore trailing commas outside strings and after objects/arrays
            }
            _ => cleaned_string.push(c),
        }
        last_char = c;
    }

    cleaned_string
}
