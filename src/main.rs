mod camera;
mod console;
mod decode;
mod encode;
mod error;

use camera::get_content_from_camera;
use clap::{Parser, Subcommand};
use error::{Result, UnwrapOrExit};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use std::io::Read;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut options = KeyStreamOptions::parse();
    options.key = options
        .password
        .as_ref()
        .map(|x| x.get_key().unwrap_or_exit());
    let result = match &options.command {
        KeyStreamCommand::Encode(_) => encode::encode(&options).await,
        KeyStreamCommand::Decode => decode::decode(&options).await,
        KeyStreamCommand::ShowKey => {
            show_key(options.key.as_ref().map(|k| k.as_ref()));
            Ok(())
        }
    };
    result.unwrap_or_exit();
}

fn show_key(key: Option<&[u8]>) {
    if let Some(key) = key {
        let hexkey = key.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        println!("{hexkey}");
    } else {
        println!("No key set");
    }
}

type ClapResult<T> = std::result::Result<T, clap::Error>;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct KeyStreamOptions {
    #[arg(short, long, help = "Input source (stdin | camera | env:<varname> | <file>)", default_value = "stdin", value_parser = InputSource::parse)]
    input: InputSource,

    #[arg(short, long, help = "Encryption password (prompt | env:<varname> | key:<hex> | <value>)", value_parser = PasswordSource::parse)]
    password: Option<PasswordSource>,

    #[clap(skip)]
    key: Option<[u8; 32]>,

    #[command(subcommand)]
    command: KeyStreamCommand,
}

impl KeyStreamOptions {
    fn encode_options(&self) -> &EncodeOptions {
        if let KeyStreamCommand::Encode(options) = &self.command {
            options
        } else {
            panic!("encode options requested for non-encode command")
        }
    }
}

#[derive(Clone, Debug)]
enum InputSource {
    Stdin,
    Camera,
    Env(String),
    File(String),
}

impl InputSource {
    fn parse(s: &str) -> ClapResult<Self> {
        if s == "stdin" {
            Ok(InputSource::Stdin)
        } else if s == "camera" {
            Ok(InputSource::Camera)
        } else if let Some(envkey) = s.strip_prefix("env:") {
            Ok(InputSource::Env(envkey.to_string()))
        } else if std::path::Path::new(s).exists() {
            Ok(InputSource::File(s.to_string()))
        } else {
            Err(error::err_invalid_input())
        }
    }

    async fn get_content(&self) -> Result<Vec<u8>> {
        match self {
            Self::Stdin => {
                let mut data = Vec::<u8>::new();
                std::io::stdin().read_to_end(&mut data)?;
                Ok(data)
            }
            Self::Camera => get_content_from_camera().await,
            Self::Env(varname) => Ok(std::env::var(varname)
                .map_err(|_| error::err_value_validation("invalid env var"))?
                .as_bytes()
                .to_vec()),
            Self::File(path) => {
                let mut file = std::fs::File::open(path)?;
                let mut data = Vec::<u8>::new();
                file.read_to_end(&mut data)?;
                Ok(data)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PasswordSource {
    Prompt,
    Env(String),
    Key([u8; 32]),
    Value(String),
}

impl PasswordSource {
    fn parse(s: &str) -> ClapResult<Self> {
        Ok(if s == "prompt" {
            Self::Prompt
        } else if let Some(envkey) = s.strip_prefix("env:") {
            Self::Env(envkey.to_string())
        } else if let Some(hexkey) = s.strip_prefix("key:") {
            let hkl = hexkey.len();
            let mut key = Vec::<u8>::new();
            let x = hkl % 2;
            for idx in 0..(hkl + 1) / 2 {
                if idx == 0 && hkl % 2 != 0 {
                    key.push(
                        u8::from_str_radix(&hexkey[0..1], 16)
                            .map_err(|_| error::err_invalid_input())?,
                    );
                } else {
                    let i1 = idx * 2 + x;
                    let i2 = i1 + 2;
                    key.push(
                        u8::from_str_radix(&hexkey[i1..i2], 16)
                            .map_err(|_| error::err_invalid_input())?,
                    );
                }
            }
            Self::Key(key.try_into().map_err(|_| error::err_invalid_input())?)
        } else {
            Self::Value(s.to_string())
        })
    }

    fn get_key(&self) -> Result<[u8; 32]> {
        let password = match self {
            Self::Prompt => console::prompt("Enter password: ", true)?,
            Self::Env(varname) => std::env::var(varname)
                .map_err(|_| error::err_value_validation("invalid env var"))?,
            Self::Key(key) => return Ok(key.to_owned()),
            Self::Value(value) => value.to_owned(),
        };
        let salt = KEYSTREAM_MAGIC.as_bytes();
        let num_iterations = 600_000;
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, num_iterations, &mut key);
        Ok(key)
    }
}

#[derive(Debug, Subcommand)]
enum KeyStreamCommand {
    Encode(EncodeOptions),
    Decode,
    ShowKey,
}

#[derive(Debug, Parser)]
struct EncodeOptions {
    #[arg(short, long, help = "Output format (png | txt)", default_value = "png", value_parser = OutputFormat::parse)]
    out_format: OutputFormat,

    #[arg(long, help = "Error correction level (L|M|Q|H)", default_value = "Q", value_parser = parse_ec_level)]
    ec_level: qr_code::EcLevel,

    #[arg(
        long,
        help = "QR codes per row, if multiple needed",
        default_value = "1"
    )]
    qr_per_row: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OutputFormat {
    Png,
    Txt,
}

impl OutputFormat {
    fn parse(s: &str) -> ClapResult<Self> {
        match s {
            "png" => Ok(OutputFormat::Png),
            "txt" => Ok(OutputFormat::Txt),
            _ => Err(error::err_value_validation(format!(
                "invalid output format {s}"
            ))),
        }
    }
}

fn parse_ec_level(s: &str) -> ClapResult<qr_code::EcLevel> {
    match s {
        "L" => Ok(qr_code::EcLevel::L),
        "M" => Ok(qr_code::EcLevel::M),
        "Q" => Ok(qr_code::EcLevel::Q),
        "H" => Ok(qr_code::EcLevel::H),
        _ => Err(crate::error::err_value_validation(format!(
            "invalid ec-level {s}"
        ))),
    }
}

pub(crate) const KEYSTREAM_MAGIC: &str = "KYST";
pub(crate) const CURRENT_VERSION: u8 = 1;
