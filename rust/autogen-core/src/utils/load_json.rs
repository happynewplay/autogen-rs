use regex::Regex;
use serde_json::{Value, from_str};
use std::error::Error;

/// Extract JSON objects from a string. Supports backtick enclosed JSON objects
pub fn extract_json_from_str(content: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let pattern = Regex::new(r"```(?:\s*([\w\+\-]+))?\n([\s\S]*?)```")?;
    let matches: Vec<_> = pattern.captures_iter(content).collect();

    let mut ret: Vec<Value> = Vec::new();

    if matches.is_empty() {
        let json_val: Value = from_str(content)?;
        ret.push(json_val);
    } else {
        for cap in matches {
            if let Some(language_match) = cap.get(1) {
                let language = language_match.as_str().trim();
                if !language.is_empty() && language.to_lowercase() != "json" {
                    return Err(format!("Expected JSON object, but found language: {}", language).into());
                }
            }
            if let Some(content_match) = cap.get(2) {
                let json_val: Value = from_str(content_match.as_str())?;
                ret.push(json_val);
            }
        }
    }

    Ok(ret)
}