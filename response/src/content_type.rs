use std::path::Path;

pub const HTML: &str = "text/html; charset=utf-8";
pub const TEXT: &str = "text/plain; charset=utf-8";
const OCTET: &str = "application/octet-stream";

pub fn get_content_type(target_path: &Path) -> &str {
    let extension = match target_path.extension() {
        Some(ext) => ext,
        _ => return OCTET,
    };

    let ext_str = match extension.to_str() {
        Some(e) => e,
        _ => return OCTET,
    };

    match ext_str {
        "aac" => "audio/aac",
        "bmp" => "image/bmp",
        "css" => "text/css; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "flac" => "audio/flac",
        "gif" => "image/gif",
        "gz" => "application/gzip",
        "html" => HTML,
        "ico" => "image/vnd.microsoft.icon",
        "jpeg" => "image/jpeg",
        "jpg" => "image/jpeg",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json",
        "m3u8" => "application/x-mpegURL",
        "m3u" => "application/x-mpegURL",
        "midi" => "audio/midi",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "mpd" => "application/dash+xml",
        "mpeg" => "video/mpeg",
        "oga" => "audio/ogg",
        "ogv" => "video/ogg",
        "otf" => "font/otf",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "txt" => TEXT,
        "tiff" => "image/tiff",
        "ts" => "video/MP2T",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        "wav" => "audio/wav",
        "weba" => "weba",
        "webm" => "video/webm",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "xml" => "application/xml; charset=utf-8",
        "zip" => "application/zip",
        _ => OCTET,
    }
}
