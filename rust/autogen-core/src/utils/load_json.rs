use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;

/// Extracts JSON objects from a string. Supports backtick-enclosed JSON objects.
///
/// # Arguments
///
/// * `content` - The string to extract JSON from.
///
/// # Returns
///
/// A `Result` containing a `Vec<Value>` of the extracted JSON objects, or an error.
pub fn extract_json_from_str(content: &str) -> Result<Vec<Value>> {
    // The regex is designed to find code blocks, optionally with a language specifier.
    // It captures the language (e.g., "json") and the content of the code block.
    let re = Regex::new(r"```(?:\s*([\w\+\-]+))?\n([\s\S]*?)```")?;
    let mut results: Vec<Value> = Vec::new();

    let captures: Vec<_> = re.captures_iter(content).collect();

    if captures.is_empty() {
        // If no code blocks are found, try to parse the entire string as a JSON object.
        match serde_json::from_str(content) {
            Ok(json_value) => {
                results.push(json_value);
            }
            Err(_) => {
                // If parsing the whole string fails, it might not be a JSON string.
                // We return an error to indicate that something was expected.
                // The python code would raise an exception here, so an error is more appropriate.
                return Err(anyhow!("No JSON found in the string and the string is not a valid JSON object."));
            }
        }
    } else {
        for cap in captures {
            // The first capture group is the language specifier. It's optional.
            if let Some(language_match) = cap.get(1) {
                let language = language_match.as_str().trim();
                if !language.is_empty() && language.to_lowercase() != "json" {
                    return Err(anyhow!(
                        "Expected JSON object, but found language: {}",
                        language
                    ));
                }
            }

            // The second capture group is the content of the code block.
            if let Some(content_match) = cap.get(2) {
                let json_str = content_match.as_str().trim();
                let json_value: Value = serde_json::from_str(json_str)?;
                results.push(json_value);
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_single_json_object() {
        let content = r#"
```json
{
    "name": "test",
    "value": 1
}
```
"#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], json!({"name": "test", "value": 1}));
    }

    #[test]
    fn test_extract_multiple_json_objects() {
        let content = r#"
Here is some JSON:
```json
{
    "name": "first",
    "value": 1
}
```
And here is another one:
```
{
    "name": "second",
    "value": 2
}
```
"#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], json!({"name": "first", "value": 1}));
        assert_eq!(result[1], json!({"name": "second", "value": 2}));
    }

    #[test]
    fn test_extract_json_without_language_specifier() {
        let content = r#"
```
{
    "name": "no_lang",
    "value": 3
}
```
"#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], json!({"name": "no_lang", "value": 3}));
    }

    #[test]
    fn test_extract_json_with_other_language_specifier() {
        let content = r#"
```python
{
    "name": "python",
    "value": 4
}
```
"#;
        let result = extract_json_from_str(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Expected JSON object, but found language: python"
        );
    }

    #[test]
    fn test_plain_json_string() {
        let content = r#"{"name": "plain", "value": 5}"#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], json!({"name": "plain", "value": 5}));
    }

    #[test]
    fn test_invalid_json_string() {
        let content = r#"{"name": "invalid", "value": 6,}"#;
        let result = extract_json_from_str(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_no_json() {
        let content = "This is just a regular string.";
        let result = extract_json_from_str(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No JSON found in the string and the string is not a valid JSON object.");
    }

    #[test]
    fn test_extract_json_with_plus_in_language() {
        let content = r#"
```json+hal
{
    "name": "json_plus",
    "value": 7
}
```
"#;
        let result = extract_json_from_str(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Expected JSON object, but found language: json+hal"
        );
    }
}