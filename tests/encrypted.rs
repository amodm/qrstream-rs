mod common;
use common::{decode, encode, rand_password_key, QRSTREAM_MAGIC, QRSTREAM_VERSION};
use lazy_static::lazy_static;

#[test]
fn test_format_txt_encode() -> Result<(), Box<dyn std::error::Error>> {
    let password = &RAND_PASSWORD;
    let data = "Hello World";
    let stdout = String::from_utf8(encode(data, "txt", password)?)?;
    assert!(stdout.starts_with(&format!("{QRSTREAM_MAGIC}/{QRSTREAM_VERSION};p=11;t=")));

    Ok(())
}

#[test]
fn test_format_txt_decode() -> Result<(), Box<dyn std::error::Error>> {
    let password = &RAND_PASSWORD;
    let data = "Hello World";
    let encrypted = encode(data, "txt", password)?;
    let decoded = decode(&encrypted, password)?;
    assert_eq!(data, decoded);

    Ok(())
}

#[test]
fn test_format_png_encode() -> Result<(), Box<dyn std::error::Error>> {
    let password = &RAND_PASSWORD;
    let data = "Hello World";
    assert!(encode(data, "png", password)?.starts_with(b"\x89PNG\x0d\x0a\x1a\x0a"));
    Ok(())
}

#[test]
fn test_format_png_decode() -> Result<(), Box<dyn std::error::Error>> {
    let password = &RAND_PASSWORD;
    let data = "Hello World";
    let encrypted = encode(data, "png", password)?;
    let decoded = decode(&encrypted, password)?;
    assert_eq!(data, decoded);

    Ok(())
}

lazy_static! {
    static ref RAND_PASSWORD: Option<String> =
        Some(format!("key:{}", rand_password_key().unwrap()));
}
