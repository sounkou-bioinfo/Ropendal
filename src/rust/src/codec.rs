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
    fn parse(name: &str) -> savvy::Result<Self> {
        match name.to_ascii_lowercase().as_str() {
            "identity" | "none" | "raw" => Ok(Self::Identity),
            "gzip" | "gz" => Ok(Self::Gzip),
            "zlib" => Ok(Self::Zlib),
            _ => Err(savvy::Error::new(&format!(
                "unsupported codec {name:?}; supported codecs are identity, gzip, and zlib"
            ))),
        }
    }
}

#[savvy]
fn opendal_codec_encode(name: &str, data: RawSexp) -> savvy::Result<savvy::Sexp> {
    let kind = CodecKind::parse(name)?;
    let out = match kind {
        CodecKind::Identity => data.to_vec(),
        CodecKind::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data.as_slice())
                .map_err(|e| savvy::Error::new(&format!("gzip encode failed: {e}")))?;
            encoder
                .finish()
                .map_err(|e| savvy::Error::new(&format!("gzip encode failed: {e}")))?
        }
        CodecKind::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data.as_slice())
                .map_err(|e| savvy::Error::new(&format!("zlib encode failed: {e}")))?;
            encoder
                .finish()
                .map_err(|e| savvy::Error::new(&format!("zlib encode failed: {e}")))?
        }
    };
    OwnedRawSexp::try_from(out)?.into()
}

#[savvy]
fn opendal_codec_decode(name: &str, data: RawSexp) -> savvy::Result<savvy::Sexp> {
    let kind = CodecKind::parse(name)?;
    let out = match kind {
        CodecKind::Identity => data.to_vec(),
        CodecKind::Gzip => {
            let mut decoder = GzDecoder::new(data.as_slice());
            let mut out = Vec::new();
            decoder
                .read_to_end(&mut out)
                .map_err(|e| savvy::Error::new(&format!("gzip decode failed: {e}")))?;
            out
        }
        CodecKind::Zlib => {
            let mut decoder = ZlibDecoder::new(data.as_slice());
            let mut out = Vec::new();
            decoder
                .read_to_end(&mut out)
                .map_err(|e| savvy::Error::new(&format!("zlib decode failed: {e}")))?;
            out
        }
    };
    OwnedRawSexp::try_from(out)?.into()
}
