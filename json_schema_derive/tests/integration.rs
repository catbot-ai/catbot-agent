use json_schema::ToJsonSchema;
use json_schema_derive::ToJsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Test Struct 1: Simple case
#[derive(Serialize, Deserialize, ToJsonSchema)]
#[gemini(name = "test_function", description = "A simple test function")]
struct SimpleStruct {
    #[gemini(description = "A string field")]
    name: String,
    #[gemini(description = "A boolean field")]
    active: bool,
}

// Test Struct 2: Complex case with optional field
#[derive(Serialize, Deserialize, ToJsonSchema)]
#[gemini(name = "complex_function", description = "A complex test function")]
struct ComplexStruct {
    #[gemini(description = "An integer ID")]
    id: i32,
    #[gemini(description = "A float value")]
    value: f64,
    #[gemini(description = "An optional note", optional)]
    note: String,
}

#[test]
fn test_simple_struct_schema() {
    let schema: Value = SimpleStruct::to_json_schema();
    let expected = json!({
        "name": "test_function",
        "description": "A simple test function",
        "parameters": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "A string field"
                },
                "active": {
                    "type": "boolean",
                    "description": "A boolean field"
                }
            },
            "required": ["name", "active"]
        }
    });
    assert_eq!(schema, expected);
}

#[test]
fn test_complex_struct_schema() {
    let schema: Value = ComplexStruct::to_json_schema();
    let expected = json!({
        "name": "complex_function",
        "description": "A complex test function",
        "parameters": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "integer",
                    "description": "An integer ID"
                },
                "value": {
                    "type": "number",
                    "description": "A float value"
                },
                "note": {
                    "type": "string",
                    "description": "An optional note"
                }
            },
            "required": ["id", "value"]
        }
    });
    assert_eq!(schema, expected);
}
