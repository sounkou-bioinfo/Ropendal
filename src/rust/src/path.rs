pub(crate) fn normalize_user_path(path: &str, directory: bool) -> Result<String, String> {
    let raw = path.trim();
    if raw == "/" || raw.is_empty() {
        return Ok(String::new());
    }

    let mut parts: Vec<&str> = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("path cannot escape filesystem root".to_string());
                }
            }
            p => parts.push(p),
        }
    }

    let mut out = parts.join("/");
    if directory && !out.is_empty() && !out.ends_with('/') {
        out.push('/');
    }
    Ok(out)
}

pub(crate) fn checked_u64(value: f64, name: &str) -> savvy::Result<u64> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(savvy::Error::new(&format!(
            "{name} must be a non-negative whole number"
        )));
    }
    if value > u64::MAX as f64 {
        return Err(savvy::Error::new(&format!("{name} is too large")));
    }
    Ok(value as u64)
}
