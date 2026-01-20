use playwright_rs::protocol::FilePayload;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_file_payload_from_path() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_document.txt");

    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();

    let payload = FilePayload::from_path(&file_path).unwrap();

    assert_eq!(payload.name, "test_document.txt");
    assert_eq!(payload.mime_type, "text/plain");
    assert_eq!(payload.buffer, b"Hello, World!");
}

#[test]
fn test_file_payload_from_path_json() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");

    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"{}").unwrap();

    let payload = FilePayload::from_path(&file_path).unwrap();

    assert_eq!(payload.name, "data.json");
    assert_eq!(payload.mime_type, "application/json");
}

#[test]
fn test_file_payload_from_file_explicit_mime() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unknown.xyz");

    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"some data").unwrap();

    let payload = FilePayload::from_file(&file_path, "application/custom").unwrap();

    assert_eq!(payload.name, "unknown.xyz");
    assert_eq!(payload.mime_type, "application/custom");
    assert_eq!(payload.buffer, b"some data");
}
