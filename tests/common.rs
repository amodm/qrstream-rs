use assert_cmd::Command;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn encode(
    data: &str,
    format: &str,
    password: &Option<String>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut args = vec![];
    if let Some(ref password) = password {
        args.push("-p");
        args.push(password);
    }
    args.extend(vec!["encode", "-o", format]);
    Ok(Command::cargo_bin(QRSTREAM_CMD)?
        .args(args)
        .write_stdin(data)
        .assert()
        .success()
        .get_output()
        .stdout
        .to_owned())
}

pub fn decode(
    encoded: &[u8],
    password: &Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut args = vec![];
    if let Some(ref password) = password {
        args.push("-p");
        args.push(password);
    }
    args.push("decode");
    Ok(String::from_utf8(
        Command::cargo_bin(QRSTREAM_CMD)?
            .args(args)
            .write_stdin(encoded)
            .assert()
            .success()
            .get_output()
            .stdout
            .to_owned(),
    )?)
}

#[allow(dead_code)]
pub fn rand_password_key() -> Result<String, Box<dyn std::error::Error>> {
    let password: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(13)
        .map(char::from)
        .collect();
    Ok(String::from_utf8(
        Command::cargo_bin(QRSTREAM_CMD)?
            .args(["-p", &password, "show-key"])
            .assert()
            .success()
            .get_output()
            .stdout
            .to_owned(),
    )?
    .trim()
    .to_owned())
}

pub const QRSTREAM_CMD: &str = env!("CARGO_PKG_NAME");
pub const QRSTREAM_MAGIC: &str = "QRST";
pub const QRSTREAM_VERSION: u8 = 1;
