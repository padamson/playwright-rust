// Filename → MIME type lookup for file uploads.
//
// Used by `ElementHandle::set_input_files` and `FilePayload::from_path`
// to set the `mimeType` field that Playwright expects on uploaded
// files. Replaces the `mime_guess` crate, whose compile-time lookup
// table covered hundreds of MIME types we'd never encounter in a
// browser-automation upload context.
//
// Anything not in this table falls back to `application/octet-stream`,
// matching `mime_guess::Mime::first_or_octet_stream()`.

use std::path::Path;

/// Returns the MIME type for a path's filename extension, or
/// `application/octet-stream` if the extension is missing or unknown.
pub(crate) fn from_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        // Images
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("tif" | "tiff") => "image/tiff",
        Some("avif") => "image/avif",

        // Documents
        Some("pdf") => "application/pdf",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        Some("odt") => "application/vnd.oasis.opendocument.text",
        Some("ods") => "application/vnd.oasis.opendocument.spreadsheet",
        Some("rtf") => "application/rtf",

        // Text and data
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("js" | "mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("yaml" | "yml") => "application/yaml",
        Some("md") => "text/markdown",

        // Archives
        Some("zip") => "application/zip",
        Some("tar") => "application/x-tar",
        Some("gz") => "application/gzip",
        Some("bz2") => "application/x-bzip2",
        Some("7z") => "application/x-7z-compressed",

        // Audio
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("flac") => "audio/flac",

        // Video
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",

        // Fonts
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",

        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions_map_to_expected_types() {
        assert_eq!(from_path(Path::new("photo.png")), "image/png");
        assert_eq!(from_path(Path::new("photo.JPG")), "image/jpeg");
        assert_eq!(from_path(Path::new("doc.pdf")), "application/pdf");
        assert_eq!(from_path(Path::new("data.json")), "application/json");
        assert_eq!(from_path(Path::new("notes.txt")), "text/plain");
        assert_eq!(from_path(Path::new("page.html")), "text/html");
        assert_eq!(from_path(Path::new("archive.zip")), "application/zip");
    }

    #[test]
    fn case_insensitive_match() {
        assert_eq!(from_path(Path::new("photo.PNG")), "image/png");
        assert_eq!(from_path(Path::new("photo.Png")), "image/png");
    }

    #[test]
    fn unknown_extension_falls_back_to_octet_stream() {
        assert_eq!(
            from_path(Path::new("mystery.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn no_extension_falls_back_to_octet_stream() {
        assert_eq!(from_path(Path::new("README")), "application/octet-stream");
        assert_eq!(
            from_path(Path::new("/etc/passwd")),
            "application/octet-stream"
        );
    }

    #[test]
    fn full_path_works() {
        assert_eq!(from_path(Path::new("/tmp/uploads/photo.png")), "image/png");
    }
}
