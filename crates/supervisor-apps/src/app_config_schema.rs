//! Vendor-neutral, strict and value-free installed Supervisor App option schema.
//! Never return stored options, defaults, URL or secret references.
//! Read only: no installation, validation, activation or reconfiguration.
use crate::{
    app_detail::{InfoError, detail_path, valid_slug},
    official_store::ApiVersion,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Secret,
    Integer,
    Number,
    Boolean,
    List,
    Group,
    Custom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigField {
    pub name: String,
    pub kind: FieldKind,
    pub required: bool,
    pub sensitive: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppConfigSchema {
    pub slug: String,
    pub fields: Vec<ConfigField>,
    pub configuration_supported: bool,
    pub values_included: bool,
    pub read_only: bool,
    pub activation_authorized: bool,
}
fn field_kind(value: &Value) -> Result<(FieldKind, bool), InfoError> {
    if value.is_object() {
        return Ok((FieldKind::Group, true));
    }
    let v = value.as_str().ok_or(InfoError::InvalidPayload)?;
    if v.len() > 256 || v.chars().any(char::is_control) {
        return Err(InfoError::InvalidPayload);
    }
    let required = !v.ends_with('?');
    let prefix = v
        .trim_end_matches('?')
        .split(['(', ':', '|'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = match prefix.as_str() {
        "str" | "string" | "email" | "url" | "device" | "port" | "match" => FieldKind::Text,
        "password" | "secret" => FieldKind::Secret,
        "int" | "positive_int" => FieldKind::Integer,
        "float" | "number" => FieldKind::Number,
        "bool" => FieldKind::Boolean,
        "list" => FieldKind::List,
        "dict" | "map" => FieldKind::Group,
        _ => FieldKind::Custom,
    };
    Ok((kind, required))
}
fn sensitive(name: &str, kind: FieldKind) -> bool {
    let lower = name.to_ascii_lowercase();
    kind == FieldKind::Secret
        || [
            "password",
            "passwd",
            "token",
            "secret",
            "credential",
            "api_key",
            "apikey",
            "auth",
            "private_key",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}
pub fn config_path(api: ApiVersion, slug: &str) -> Result<String, InfoError> {
    detail_path(api, slug)
}
pub fn parse_config_schema(body: &[u8], slug: &str) -> Result<AppConfigSchema, InfoError> {
    if !valid_slug(slug) {
        return Err(InfoError::UnsafeSlug);
    }
    if body.len() > 512 * 1024 {
        return Err(InfoError::TooLarge);
    }
    let root = serde_json::from_slice::<crate::NoDuplicates>(body)
        .map_err(|_| InfoError::InvalidPayload)?
        .0;
    if root.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(InfoError::InvalidPayload);
    }
    let data = root
        .get("data")
        .and_then(Value::as_object)
        .ok_or(InfoError::InvalidPayload)?;
    if data.get("slug").and_then(Value::as_str) != Some(slug) {
        return Err(InfoError::MismatchedSlug);
    }
    let schema = data.get("schema").ok_or(InfoError::InvalidPayload)?;
    let mut fields = Vec::new();
    let configuration_supported = match schema {
        Value::Bool(false) => false,
        Value::Object(map) => {
            if map.len() > 128 {
                return Err(InfoError::TooLarge);
            }
            for (name, value) in map {
                if name.is_empty()
                    || name.len() > 64
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                {
                    return Err(InfoError::InvalidPayload);
                }
                let (kind, required) = field_kind(value)?;
                fields.push(ConfigField {
                    name: name.clone(),
                    kind,
                    required,
                    sensitive: sensitive(name, kind),
                });
            }
            true
        }
        _ => return Err(InfoError::InvalidPayload),
    };
    Ok(AppConfigSchema {
        slug: slug.to_owned(),
        fields,
        configuration_supported,
        values_included: false,
        read_only: true,
        activation_authorized: false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn body(slug: &str, schema: Value) -> Vec<u8> {
        json!({"result":"ok","data":{"slug":slug,"schema":schema,
        "options":{"access_token":"SUPERSECRET_PASS_DO_NOT_EXPOSE",
            "dynamic_list":["private"],"group":{"password":"secret"}},
        "network":{"443/tcp":443},"ingress_url":"/hassio/secret"}})
        .to_string()
        .into_bytes()
    }
    #[test]
    fn arbitrary_vendors_and_never_a_value() {
        let raw = body(
            "unfamiliar_vendor_917",
            json!({
              "access_token":"password","foo_api_key":"str?", "slots":"int",
              "dynamic_list":"list(str)","group":{"oauth":{"password":"bad"}},"testing":"unknown(secret)"
            }),
        );
        let v = parse_config_schema(&raw, "unfamiliar_vendor_917").unwrap();
        let out = serde_json::to_string(&v).unwrap();
        assert!(v.read_only && !v.values_included && !v.activation_authorized);
        assert_eq!(v.fields.len(), 6);
        assert!(
            v.fields
                .iter()
                .any(|x| x.name == "foo_api_key" && x.sensitive && !x.required)
        );
        assert!(
            v.fields
                .iter()
                .any(|x| x.name == "access_token" && x.kind == FieldKind::Secret)
        );
        assert!(!out.contains("SUPERSECRET_PASS_DO_NOT_EXPOSE") && !out.contains("bad"));
        assert!(!out.contains("ingress_url") && !out.contains("private"));
    }
    #[test]
    fn no_form_mode_not_secret_options() {
        let v = parse_config_schema(&body("vendor_12", json!(false)), "vendor_12").unwrap();
        assert!(!v.configuration_supported && v.fields.is_empty() && !v.values_included);
    }
    #[test]
    fn invalid_slug_and_mismatched_payload_rejected() {
        for slug in ["../other", "?", "vendor%2fother", "éx", ""] {
            assert_eq!(
                config_path(ApiVersion::V1, slug),
                Err(InfoError::UnsafeSlug)
            );
        }
        for v in [ApiVersion::V1, ApiVersion::V2] {
            assert_eq!(
                config_path(v, "vendor_222").unwrap(),
                match v {
                    ApiVersion::V1 => "/addons/vendor_222/info",
                    ApiVersion::V2 => "/v2/apps/vendor_222/info",
                }
            );
        }
        assert_eq!(
            parse_config_schema(&body("different", json!({})), "vendor_222"),
            Err(InfoError::MismatchedSlug)
        );
        assert!(
            parse_config_schema(
                br#"{"result":"ok","data":{"slug":"ok","schema":{},"schema":{}}}"#,
                "ok"
            )
            .is_err()
        );
    }
    #[test]
    fn dangerous_or_oversized_schema_rejected() {
        for schema in [
            json!(true),
            json!(null),
            json!({"foo/bar":"str"}),
            json!({"evil\n":"str"}),
            json!({"ok":true}),
            json!({"ok":"str\nINJECT"}),
        ] {
            assert!(parse_config_schema(&body("demo", schema), "demo").is_err());
        }
        assert_eq!(
            parse_config_schema(&vec![b'a'; 512 * 1024 + 1], "demo"),
            Err(InfoError::TooLarge)
        );
        let mut big = serde_json::Map::new();
        for i in 0..129 {
            big.insert(format!("f_{i}"), json!("str"));
        }
        assert_eq!(
            parse_config_schema(&body("demo", Value::Object(big)), "demo"),
            Err(InfoError::TooLarge)
        );
    }
}
