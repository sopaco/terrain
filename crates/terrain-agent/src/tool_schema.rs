//! JSON Schema helpers for OpenAI-compatible providers (e.g. LM Studio) that require
//! `parameters.properties` on every function tool.

use serde_json::{Value, json};

/// Ensure object tool schemas include `type` and `properties` (LM Studio rejects missing `properties`).
pub fn ensure_tool_parameters_object(schema: Value) -> Value {
    let mut schema = schema;
    fix_object_schema(&mut schema);
    schema
}

fn fix_object_schema(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };

    let is_object = obj.get("type").and_then(|t| t.as_str()) == Some("object")
        || obj.contains_key("properties")
        || obj.get("additionalProperties").is_some();

    if is_object {
        if !obj.contains_key("type") {
            obj.insert("type".to_string(), json!("object"));
        }
        if !obj.contains_key("properties") {
            obj.insert("properties".to_string(), json!({}));
        }
    }

    recurse_subschemas(schema, fix_object_schema);
}

fn recurse_subschemas(schema: &mut Value, visit: fn(&mut Value)) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };

    if let Some(props) = obj.get_mut("properties") {
        if let Some(props_obj) = props.as_object_mut() {
            for value in props_obj.values_mut() {
                visit(value);
            }
        }
    }

    for keyword in ["items", "additionalProperties", "not"] {
        if let Some(sub) = obj.get_mut(keyword) {
            if sub.is_object() {
                visit(sub);
            }
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(arr) = obj.get_mut(keyword).and_then(|v| v.as_array_mut()) {
            for item in arr.iter_mut() {
                visit(item);
            }
        }
    }

    for keyword in ["$defs", "definitions", "patternProperties"] {
        if let Some(map) = obj.get_mut(keyword).and_then(|v| v.as_object_mut()) {
            for value in map.values_mut() {
                visit(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::{
        JsonSchema,
        generate::{SchemaGenerator, SchemaSettings},
    };
    use serde::Deserialize;
    use serde::Serialize;

    fn generate_parameters_schema<T>() -> serde_json::Value
    where
        T: JsonSchema + Serialize,
    {
        let settings = SchemaSettings::openapi3().with(|s| {
            s.inline_subschemas = true;
            s.meta_schema = None;
        });
        let generator = SchemaGenerator::new(settings);
        let mut schema = generator.into_root_schema_for::<T>();
        if let Some(object) = schema.as_object_mut() {
            object.remove("title");
        }
        serde_json::to_value(schema).unwrap()
    }

    fn parameters_schema_for<T>() -> Value
    where
        T: JsonSchema + Serialize,
    {
        ensure_tool_parameters_object(generate_parameters_schema::<T>())
    }

    #[derive(Debug, Deserialize, Serialize, JsonSchema)]
    struct EmptyArgs {}

    #[derive(Debug, Deserialize, Serialize, JsonSchema)]
    struct ProjectSlugArgs {
        project: String,
    }

    #[test]
    fn empty_args_has_properties() {
        let schema = parameters_schema_for::<EmptyArgs>();
        assert!(schema.get("properties").is_some());
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn project_slug_args_has_properties() {
        let schema = parameters_schema_for::<ProjectSlugArgs>();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("project").is_some());
    }
}
