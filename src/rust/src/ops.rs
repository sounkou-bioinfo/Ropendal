use opendal::options::WriteOptions;
use opendal::{Error, ErrorKind, Operator};

pub(crate) async fn read_bytes(
    op: Operator,
    path: String,
    offset: u64,
    size: Option<u64>,
) -> Result<Vec<u8>, opendal::Error> {
    let buf = match size {
        Some(n) => op
            .read_with(&path)
            .range(offset..offset.saturating_add(n))
            .await?,
        None if offset == 0 => op.read(&path).await?,
        None => op.read_with(&path).range(offset..).await?,
    };
    Ok(buf.to_vec())
}

pub(crate) async fn write_bytes(
    op: Operator,
    path: String,
    bytes: Vec<u8>,
    create_only: bool,
    append: bool,
) -> Result<(), opendal::Error> {
    if create_only {
        match op.stat(&path).await {
            Ok(_) => {
                return Err(Error::new(ErrorKind::AlreadyExists, "target already exists")
                    .with_operation("write")
                    .with_context("path", &path));
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    let mut opts = WriteOptions::default();
    opts.if_not_exists = create_only;
    opts.append = append;
    op.write_options(&path, bytes, opts).await?;
    Ok(())
}
