use std::sync::Arc;

use opendal::Operator;

#[derive(Clone)]
pub(crate) struct NativeFs {
    pub(crate) op: Operator,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) scheme: String,
    pub(crate) root: String,
}

pub(crate) fn build_runtime(threads: Option<usize>) -> savvy::Result<Arc<tokio::runtime::Runtime>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(threads.unwrap_or(2))
        .thread_name("ropendal-worker")
        .build()
        .map(Arc::new)
        .map_err(|e| savvy::Error::new(&format!("cannot create async runtime: {e}")))
}

pub(crate) fn init_registry() {
    opendal::init_default_registry();
}
