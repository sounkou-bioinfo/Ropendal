use std::fs;

use savvy::{OwnedListSexp, OwnedStringSexp, savvy};
use serde_json::Value;

use crate::r_values::str_scalar;

/// Explicit credential provider.
/// @export
#[savvy]
pub struct OpendalCredentialProvider {
    service: String,
    config: Vec<(String, String)>,
    source: String,
    method: String,
}

#[savvy]
impl OpendalCredentialProvider {
    /// Build a Google Drive credential provider from explicit token fields.
    /// @export
    fn gdrive(
        access_token: &str,
        refresh_token: &str,
        client_id: &str,
        client_secret: &str,
        source: &str,
    ) -> savvy::Result<Self> {
        let cfg = build_gdrive_config(access_token, refresh_token, client_id, client_secret)?;
        let method = if access_token.is_empty() {
            "refresh_token"
        } else {
            "access_token"
        };
        Ok(Self {
            service: "gdrive".to_string(),
            config: cfg,
            source: checked_scalar(source, "source")?.to_string(),
            method: method.to_string(),
        })
    }

    /// Build an S3-compatible credential provider from explicit key fields.
    /// @export
    fn s3(
        access_key_id: &str,
        secret_access_key: &str,
        session_token: &str,
        region: &str,
        source: &str,
    ) -> savvy::Result<Self> {
        Ok(Self {
            service: "s3".to_string(),
            config: build_s3_config(access_key_id, secret_access_key, session_token, region)?,
            source: checked_scalar(source, "source")?.to_string(),
            method: "access_key".to_string(),
        })
    }

    /// Build a Google Drive credential provider from a gdrive3 account directory.
    /// @export
    fn gdrive3(secret_json: &str, tokens_json: &str, scope: &str) -> savvy::Result<Self> {
        let secret_json = checked_scalar(secret_json, "secret_json")?;
        let tokens_json = checked_scalar(tokens_json, "tokens_json")?;
        let scope = checked_scalar(scope, "scope")?;
        let secret_text = fs::read_to_string(secret_json)
            .map_err(|e| savvy::Error::new(&format!("cannot read secret_json: {e}")))?;
        let tokens_text = fs::read_to_string(tokens_json)
            .map_err(|e| savvy::Error::new(&format!("cannot read tokens_json: {e}")))?;
        let secret_value: Value = serde_json::from_str(&secret_text)
            .map_err(|e| savvy::Error::new(&format!("cannot parse secret_json: {e}")))?;
        let tokens_value: Value = serde_json::from_str(&tokens_text)
            .map_err(|e| savvy::Error::new(&format!("cannot parse tokens_json: {e}")))?;
        let (client_id, client_secret) = secret_fields(&secret_value).map_err(savvy::Error::new)?;
        let refresh_token = refresh_from_tokens(&tokens_value, scope).map_err(savvy::Error::new)?;
        Self::gdrive(
            "",
            &refresh_token,
            &client_id,
            &client_secret,
            &format!("gdrive3:{}", parent_dir(secret_json)),
        )
    }

    /// Return supported service schemes.
    /// @export
    fn schemes(&self) -> savvy::Result<savvy::Sexp> {
        let mut out = OwnedStringSexp::new(1)?;
        out.set_elt(0, &self.service)?;
        out.into()
    }

    /// Materialize service config for OpenDAL.
    /// @export
    fn config(&self, service: &str) -> savvy::Result<savvy::Sexp> {
        let service = checked_scalar(service, "service")?;
        if service != self.service {
            return Err(savvy::Error::new(&format!(
                "credential provider for {} cannot be used with {service}",
                self.service
            )));
        }
        named_list(&self.config)
    }

    /// Return a redacted credential summary.
    /// @export
    fn summary(&self) -> savvy::Result<savvy::Sexp> {
        named_list(&[
            ("service".to_string(), self.service.clone()),
            ("source".to_string(), self.source.clone()),
            ("method".to_string(), self.method.clone()),
            ("secrets".to_string(), "<redacted>".to_string()),
        ])
    }
}

fn checked_scalar<'a>(value: &'a str, name: &str) -> savvy::Result<&'a str> {
    if value.is_empty() {
        Err(savvy::Error::new(&format!("{name} is required")))
    } else {
        Ok(value)
    }
}

fn build_s3_config(
    access_key_id: &str,
    secret_access_key: &str,
    session_token: &str,
    region: &str,
) -> savvy::Result<Vec<(String, String)>> {
    checked_scalar(access_key_id, "access_key_id")?;
    checked_scalar(secret_access_key, "secret_access_key")?;
    let mut out = vec![
        ("access_key_id".to_string(), access_key_id.to_string()),
        (
            "secret_access_key".to_string(),
            secret_access_key.to_string(),
        ),
    ];
    if !session_token.is_empty() {
        out.push(("session_token".to_string(), session_token.to_string()));
    }
    if !region.is_empty() {
        out.push(("region".to_string(), region.to_string()));
    }
    Ok(out)
}

fn build_gdrive_config(
    access_token: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> savvy::Result<Vec<(String, String)>> {
    match (!access_token.is_empty(), !refresh_token.is_empty()) {
        (true, true) => Err(savvy::Error::new(
            "use only one of access_token or refresh_token",
        )),
        (false, false) => Err(savvy::Error::new(
            "access_token or refresh_token is required",
        )),
        (true, false) => Ok(vec![("access_token".to_string(), access_token.to_string())]),
        (false, true) => {
            checked_scalar(client_id, "client_id")?;
            checked_scalar(client_secret, "client_secret")?;
            Ok(vec![
                ("refresh_token".to_string(), refresh_token.to_string()),
                ("client_id".to_string(), client_id.to_string()),
                ("client_secret".to_string(), client_secret.to_string()),
            ])
        }
    }
}

fn named_list(values: &[(String, String)]) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(values.len(), true)?;
    for (i, (name, value)) in values.iter().enumerate() {
        out.set_name_and_value(i, name, str_scalar(value)?)?;
    }
    out.into()
}

fn secret_fields(secret: &Value) -> Result<(String, String), String> {
    let inner = secret
        .get("installed")
        .or_else(|| secret.get("web"))
        .unwrap_or(secret);
    let client_id = string_field(inner, "client_id")?;
    let client_secret = string_field(inner, "client_secret")?;
    Ok((client_id.to_string(), client_secret.to_string()))
}

fn refresh_from_tokens(tokens: &Value, scope: &str) -> Result<String, String> {
    if let Some(token) = tokens.get("token").unwrap_or(tokens).get("refresh_token") {
        return token
            .as_str()
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .ok_or_else(|| "tokens_json refresh_token must be a non-empty string".to_string());
    }

    let entries = tokens
        .as_array()
        .ok_or_else(|| "tokens_json must be an object or array".to_string())?;
    for entry in entries {
        let scope_ok = entry
            .get("scopes")
            .and_then(Value::as_array)
            .map(|scopes| {
                scopes.is_empty()
                    || scopes
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|candidate| candidate == scope)
            })
            .unwrap_or(true);
        if !scope_ok {
            continue;
        }
        let token = entry.get("token").unwrap_or(entry);
        if let Some(refresh) = token.get("refresh_token").and_then(Value::as_str) {
            if !refresh.is_empty() {
                return Ok(refresh.to_string());
            }
        }
    }
    Err("tokens_json does not contain a matching refresh_token".to_string())
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("secret_json must contain {field}"))
}

fn parent_dir(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_plain_secret() {
        let secret = json!({"client_id":"client", "client_secret":"secret"});
        assert_eq!(
            secret_fields(&secret).unwrap(),
            ("client".to_string(), "secret".to_string())
        );
    }

    #[test]
    fn parses_installed_secret() {
        let secret = json!({"installed":{"client_id":"client", "client_secret":"secret"}});
        assert_eq!(secret_fields(&secret).unwrap().0, "client");
    }

    #[test]
    fn selects_matching_refresh_token() {
        let tokens = json!([
            {"scopes":["other"], "token":{"refresh_token":"wrong"}},
            {"scopes":["https://www.googleapis.com/auth/drive"], "token":{"refresh_token":"right"}}
        ]);
        assert_eq!(
            refresh_from_tokens(&tokens, "https://www.googleapis.com/auth/drive").unwrap(),
            "right"
        );
    }

    #[test]
    fn rejects_missing_refresh_token() {
        let tokens = json!([{"scopes":["other"], "token":{}}]);
        assert!(refresh_from_tokens(&tokens, "drive").is_err());
    }

    #[test]
    fn builds_s3_config() {
        let cfg = build_s3_config("access", "secret", "session", "region").unwrap();
        assert_eq!(cfg[0], ("access_key_id".to_string(), "access".to_string()));
        assert_eq!(
            cfg[1],
            ("secret_access_key".to_string(), "secret".to_string())
        );
        assert_eq!(cfg[2], ("session_token".to_string(), "session".to_string()));
        assert_eq!(cfg[3], ("region".to_string(), "region".to_string()));
    }

    #[test]
    fn rejects_missing_s3_secret() {
        assert!(build_s3_config("access", "", "", "").is_err());
    }

    #[test]
    fn builds_refresh_config() {
        let cfg = build_gdrive_config("", "refresh", "client", "secret").unwrap();
        assert_eq!(cfg[0], ("refresh_token".to_string(), "refresh".to_string()));
        assert_eq!(cfg.len(), 3);
    }

    #[test]
    fn rejects_ambiguous_token_config() {
        assert!(build_gdrive_config("access", "refresh", "client", "secret").is_err());
    }
}
