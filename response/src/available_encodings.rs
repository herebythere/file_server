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
