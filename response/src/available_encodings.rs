use hyper::body::Incoming;
use hyper::header::ACCEPT_ENCODING;
use hyper::http::Request;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AvailableEncodings {
    gzip: bool,
    deflate: bool,
    br: bool,
    zstd: bool,
}

impl AvailableEncodings {
    pub fn from(potential_encodings: &Vec<String>) -> AvailableEncodings {
        let mut av_enc = AvailableEncodings {
            gzip: false,
            deflate: false,
            br: false,
            zstd: false,
        };

        for encoding in potential_encodings {
            match encoding.as_str() {
                "gzip" => av_enc.gzip = true,
                "deflate" => av_enc.deflate = true,
                "br" => av_enc.br = true,
                "zstd" => av_enc.zstd = true,
                _ => {}
            }
        }

        av_enc
    }

    pub fn encoding_is_available(&self, encoding: &str) -> bool {
        match encoding {
            "gzip" => self.gzip,
            "deflate" => self.deflate,
            "br" => self.br,
            "zstd" => self.zstd,
            _ => false,
        }
    }
}

pub fn get_encoded_ext(encoding: &str) -> Option<&str> {
    match encoding {
        "gzip" => Some("gz"),
        "zstd" => Some("zst"),
        "br" => Some("br"),
        "deflate" => Some("zz"),
        _ => None,
    }
}

pub fn get_encodings(
    req: &Request<Incoming>,
    available_encodings: &AvailableEncodings,
) -> Vec<String> {
    let mut encodings = Vec::new();

    let accept_encoding_header = match req.headers().get(ACCEPT_ENCODING) {
        Some(enc) => enc,
        _ => return encodings,
    };

    let encoding_str = match accept_encoding_header.to_str() {
        Ok(s) => s,
        _ => return encodings,
    };

    for encoding in encoding_str.split(",") {
        let trimmed = encoding.trim();
        if available_encodings.encoding_is_available(trimmed) {
            encodings.push(trimmed.to_string());
        }
    }

    return encodings;
}

pub fn add_extension(filepath: &PathBuf, encoding: &str) -> Option<PathBuf> {
    let enc_ext = match get_encoded_ext(encoding) {
        Some(enc) => enc,
        _ => return None,
    };

    let mut ext_path = filepath.clone();
    ext_path.add_extension(enc_ext);

    Some(ext_path)
}
