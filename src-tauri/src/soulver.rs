use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::json;
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use numbat::{module_importer::BuiltinModuleImporter, resolver::CodeSource, Context};

#[cfg(target_os = "macos")]
static NUMBAT_CONTEXT: Lazy<Mutex<Context>> = Lazy::new(|| {
    let ctx = Context::new(BuiltinModuleImporter::default());
    // Pre-fetch exchange rates in the background if needed
    // This is optional and won't block the initialization
    Mutex::new(ctx)
});

#[cfg(not(target_os = "macos"))]
pub fn initialize(_soulver_core_path: &str) {
    // No-op for non-macOS platforms for now
}

#[cfg(target_os = "macos")]
pub fn initialize(_soulver_core_path: &str) {
    // Initialize the context when first called
    let _ = &*NUMBAT_CONTEXT;
}

#[tauri::command]
#[cfg(target_os = "macos")]
pub fn calculate_soulver(expression: String) -> Result<String, String> {
    let mut ctx = NUMBAT_CONTEXT.lock().map_err(|e| e.to_string())?;

    match ctx.interpret(&expression, CodeSource::Text) {
        Ok((_, result)) => {
            match &result {
                numbat::InterpreterResult::Value(_value) => {
                    // Convert the result to a string representation
                    let value_string = if let Some(value_str) = result.value_as_string() {
                        value_str.to_string()
                    } else {
                        // Fallback to debug representation if no string representation is available
                        format!("{:?}", result)
                    };

                    // Return JSON format to maintain compatibility with existing frontend
                    let response = json!({
                        "value": value_string,
                        "type": "Number",
                        "error": null
                    });
                    Ok(response.to_string())
                }
                numbat::InterpreterResult::Continue => {
                    // This shouldn't happen for simple expressions
                    let response = json!({
                        "value": "",
                        "type": "Continue",
                        "error": "Expression incomplete"
                    });
                    Ok(response.to_string())
                }
            }
        }
        Err(error) => {
            // Handle errors by returning them in the expected JSON format
            let error_message = format!("{}", error);
            let response = json!({
                "value": "",
                "type": "Error",
                "error": error_message
            });
            Ok(response.to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn calculate_soulver(expression: String) -> Result<String, String> {
    // Fallback implementation for non-macOS platforms
    // This is a simple calculator that handles basic arithmetic
    use std::str::FromStr;

    // Try to evaluate simple arithmetic expressions
    let cleaned = expression.trim().replace(" ", "");

    // Handle simple addition
    if let Some(pos) = cleaned.find('+') {
        let (left, right) = cleaned.split_at(pos);
        let right = &right[1..]; // Skip the '+'
        if let (Ok(a), Ok(b)) = (f64::from_str(left), f64::from_str(right)) {
            let result = a + b;
            let response = json!({
                "value": result.to_string(),
                "type": "Number",
                "error": null
            });
            return Ok(response.to_string());
        }
    }

    // Handle simple subtraction
    if let Some(pos) = cleaned.rfind('-') {
        if pos > 0 {
            // Make sure it's not a negative sign at the start
            let (left, right) = cleaned.split_at(pos);
            let right = &right[1..]; // Skip the '-'
            if let (Ok(a), Ok(b)) = (f64::from_str(left), f64::from_str(right)) {
                let result = a - b;
                let response = json!({
                    "value": result.to_string(),
                    "type": "Number",
                    "error": null
                });
                return Ok(response.to_string());
            }
        }
    }

    // Handle simple multiplication
    if let Some(pos) = cleaned.find('*') {
        let (left, right) = cleaned.split_at(pos);
        let right = &right[1..]; // Skip the '*'
        if let (Ok(a), Ok(b)) = (f64::from_str(left), f64::from_str(right)) {
            let result = a * b;
            let response = json!({
                "value": result.to_string(),
                "type": "Number",
                "error": null
            });
            return Ok(response.to_string());
        }
    }

    // Handle simple division
    if let Some(pos) = cleaned.find('/') {
        let (left, right) = cleaned.split_at(pos);
        let right = &right[1..]; // Skip the '/'
        if let (Ok(a), Ok(b)) = (f64::from_str(left), f64::from_str(right)) {
            if b != 0.0 {
                let result = a / b;
                let response = json!({
                    "value": result.to_string(),
                    "type": "Number",
                    "error": null
                });
                return Ok(response.to_string());
            } else {
                let response = json!({
                    "value": "",
                    "type": "Error",
                    "error": "Division by zero"
                });
                return Ok(response.to_string());
            }
        }
    }

    // Try to parse as a simple number
    if let Ok(num) = f64::from_str(&cleaned) {
        let response = json!({
            "value": num.to_string(),
            "type": "Number",
            "error": null
        });
        return Ok(response.to_string());
    }

    // If we can't parse it, return an error
    let response = json!({
        "value": "",
        "type": "Error",
        "error": "Unable to evaluate expression"
    });
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_soulver_basic_arithmetic() {
        let result = calculate_soulver("10 + 5".to_string()).unwrap();

        // Parse the JSON response
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Check that we got a valid result
        assert!(parsed["error"].is_null());
        assert_eq!(parsed["type"], "Number");

        // The value should be "15" (as string)
        let value_str = parsed["value"].as_str().unwrap();
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_soulver_subtraction() {
        let result = calculate_soulver("20 - 8".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"].is_null());

        let value_str = parsed["value"].as_str().unwrap();
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_soulver_multiplication() {
        let result = calculate_soulver("6 * 7".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"].is_null());

        let value_str = parsed["value"].as_str().unwrap();
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_soulver_division() {
        let result = calculate_soulver("15 / 3".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"].is_null());

        let value_str = parsed["value"].as_str().unwrap();
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_soulver_division_by_zero() {
        let result = calculate_soulver("10 / 0".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!parsed["error"].is_null());
        assert_eq!(parsed["error"], "Division by zero");
    }

    #[test]
    fn test_calculate_soulver_invalid_expression() {
        let result = calculate_soulver("invalid expression".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!parsed["error"].is_null());
    }

    #[test]
    fn test_calculate_soulver_simple_number() {
        let result = calculate_soulver("42".to_string()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"].is_null());

        let value_str = parsed["value"].as_str().unwrap();
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 42.0).abs() < 0.001);
    }
}
