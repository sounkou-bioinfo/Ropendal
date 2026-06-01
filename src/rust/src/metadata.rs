use opendal::Metadata;
use savvy::OwnedListSexp;

use crate::r_values::{real_scalar, set_str_or_null, str_scalar};

fn metadata_type(meta: &Metadata) -> &'static str {
    if meta.is_file() {
        "file"
    } else if meta.is_dir() {
        "dir"
    } else {
        "unknown"
    }
}

pub(crate) fn metadata_list(path: &str, meta: &Metadata) -> savvy::Result<savvy::Sexp> {
    let mut out = OwnedListSexp::new(8, true)?;
    out.set_name_and_value(0, "path", str_scalar(path)?)?;
    out.set_name_and_value(1, "type", str_scalar(metadata_type(meta))?)?;
    out.set_name_and_value(2, "size", real_scalar(meta.content_length() as f64)?)?;
    set_str_or_null(&mut out, 3, "etag", meta.etag())?;
    let last_modified = meta.last_modified().map(|v| v.to_string());
    set_str_or_null(&mut out, 4, "last_modified", last_modified.as_deref())?;
    set_str_or_null(&mut out, 5, "version", meta.version())?;
    set_str_or_null(&mut out, 6, "content_type", meta.content_type())?;
    set_str_or_null(&mut out, 7, "content_encoding", meta.content_encoding())?;
    out.into()
}
