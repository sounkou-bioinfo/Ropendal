use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use savvy::{OwnedRawSexp, RawSexp, savvy};

#[derive(Clone, Copy)]
enum CodecKind {
    Identity,
    Gzip,
    Zlib,
}

impl CodecKind {
    fn parse(name: &str) -> Result<Self, String> {
        match name.to_ascii_lowercase().as_str() {
            "identity" | "none" | "raw" => Ok(Self::Identity),
            "gzip" | "gz" => Ok(Self::Gzip),
            "zlib" => Ok(Self::Zlib),
            _ => Err(format!(
                "unsupported codec {name:?}; supported codecs are identity, gzip, and zlib"
            )),
        }
    }
}

pub(crate) fn encode_bytes(name: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let kind = CodecKind::parse(name)?;
    match kind {
        CodecKind::Identity => Ok(data.to_vec()),
        CodecKind::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data)
                .map_err(|e| format!("gzip encode failed: {e}"))?;
            encoder
                .finish()
                .map_err(|e| format!("gzip encode failed: {e}"))
        }
        CodecKind::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data)
                .map_err(|e| format!("zlib encode failed: {e}"))?;
            encoder
                .finish()
                .map_err(|e| format!("zlib encode failed: {e}"))
        }
    }
}

pub(crate) fn decode_bytes(name: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let kind = CodecKind::parse(name)?;
    match kind {
        CodecKind::Identity => Ok(data.to_vec()),
        CodecKind::Gzip => {
            let mut decoder = GzDecoder::new(data);
            let mut out = Vec::new();
            decoder
                .read_to_end(&mut out)
                .map_err(|e| format!("gzip decode failed: {e}"))?;
            Ok(out)
        }
        CodecKind::Zlib => {
            let mut decoder = ZlibDecoder::new(data);
            let mut out = Vec::new();
            decoder
                .read_to_end(&mut out)
                .map_err(|e| format!("zlib decode failed: {e}"))?;
            Ok(out)
        }
    }
}

#[savvy]
fn opendal_codec_encode(name: &str, data: RawSexp) -> savvy::Result<savvy::Sexp> {
    let out = encode_bytes(name, data.as_slice()).map_err(|e| savvy::Error::new(&e))?;
    OwnedRawSexp::try_from(out)?.into()
}

#[savvy]
fn opendal_codec_decode(name: &str, data: RawSexp) -> savvy::Result<savvy::Sexp> {
    let out = decode_bytes(name, data.as_slice()).map_err(|e| savvy::Error::new(&e))?;
    OwnedRawSexp::try_from(out)?.into()
}
