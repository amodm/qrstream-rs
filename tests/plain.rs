mod common;
use common::{decode, encode, QRSTREAM_MAGIC, QRSTREAM_VERSION};

#[test]
fn test_format_txt_encode() -> Result<(), Box<dyn std::error::Error>> {
    let data = "Hello World";
    let stdout = String::from_utf8(encode(data, "txt", &None)?)?;
    assert!(stdout.starts_with(&format!("{QRSTREAM_MAGIC}/{QRSTREAM_VERSION};p=11;t=")));

    Ok(())
}

#[test]
fn test_format_txt_decode() -> Result<(), Box<dyn std::error::Error>> {
    let data = "Hello World";
    let encrypted = encode(data, "txt", &None)?;
    let decoded = decode(&encrypted, &None)?;
    assert_eq!(data, decoded);

    Ok(())
}

#[test]
fn test_format_png_encode() -> Result<(), Box<dyn std::error::Error>> {
    let data = "Hello World";
    assert!(encode(data, "png", &None)?.starts_with(b"\x89PNG\x0d\x0a\x1a\x0a"));
    Ok(())
}

#[test]
fn test_format_png_decode() -> Result<(), Box<dyn std::error::Error>> {
    let data = "Hello World";
    let encrypted = encode(data, "png", &None)?;
    let decoded = decode(&encrypted, &None)?;
    assert_eq!(data, decoded);

    Ok(())
}
