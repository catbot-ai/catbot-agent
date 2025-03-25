pub use serde_json::json;
pub use serde_json::Value;

pub trait ToJsonSchema {
    fn to_json_schema() -> Value;
}
