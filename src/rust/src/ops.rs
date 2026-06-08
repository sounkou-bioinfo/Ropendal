use opendal::options::{ReadOptions, WriteOptions};
use opendal::{Buffer, Error, ErrorKind, Operator};

#[derive(Clone, Copy, Default)]
pub(crate) struct ReadTuning {
    pub(crate) read_concurrency: Option<usize>,
    pub(crate) chunk_size: Option<usize>,
    pub(crate) coalesce_gap: Option<usize>,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct WriteTuning {
    pub(crate) write_concurrency: Option<usize>,
    pub(crate) chunk_size: Option<usize>,
}

pub(crate) async fn read_bytes_with(
    op: Operator,
    path: String,
    offset: u64,
    size: Option<u64>,
    tuning: ReadTuning,
) -> Result<Buffer, opendal::Error> {
    let mut opts = ReadOptions::default();
    if let Some(n) = size {
        opts.range = (offset..offset.saturating_add(n)).into();
    } else if offset != 0 {
        opts.range = (offset..).into();
    }
    if let Some(concurrent) = tuning.read_concurrency {
        opts.concurrent = concurrent;
    }
    if let Some(chunk_size) = tuning.chunk_size {
        opts.chunk = Some(chunk_size);
    }
    if let Some(gap) = tuning.coalesce_gap {
        opts.gap = Some(gap);
    }
    op.read_options(&path, opts).await
}

pub(crate) async fn write_bytes_with(
    op: Operator,
    path: String,
    bytes: Buffer,
    create_only: bool,
    append: bool,
    tuning: WriteTuning,
) -> Result<(), opendal::Error> {
    if create_only {
        match op.stat(&path).await {
            Ok(_) => {
                return Err(
                    Error::new(ErrorKind::AlreadyExists, "target already exists")
                        .with_operation("write")
                        .with_context("path", &path),
                );
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    let mut opts = WriteOptions::default();
    opts.if_not_exists = create_only;
    opts.append = append;
    if let Some(concurrent) = tuning.write_concurrency {
        opts.concurrent = concurrent;
    }
    if let Some(chunk_size) = tuning.chunk_size {
        opts.chunk = Some(chunk_size);
    }
    op.write_options(&path, bytes, opts).await?;
    Ok(())
}
