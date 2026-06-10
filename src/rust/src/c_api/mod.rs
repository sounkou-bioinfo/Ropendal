#![allow(non_camel_case_types)]
#![allow(unsafe_op_in_unsafe_fn)]

mod aio;
mod codec;
mod error;
mod fs;
mod ops;
mod store;
mod sync;
mod types;

pub(crate) use error::{CErrorInfo, c_error_from_opendal, c_str, set_c_error};
pub(crate) use types::*;
