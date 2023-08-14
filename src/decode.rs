use crate::error::{err_invalid_input, err_value_validation};
use crate::{KeyStreamOptions, CURRENT_VERSION, KEYSTREAM_MAGIC};

use super::error::Result;

use aes_gcm::Nonce;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::io::Write;

pub(crate) async fn decode(options: &super::KeyStreamOptions) -> Result<()> {
    let recvd_data = options.input.get_content().await?;
    let is_png = recvd_data.len() > 8 && recvd_data.starts_with(b"\x89PNG\x0d\x0a\x1a\x0a");
    let raw_text = if is_png {
        let image = image::load_from_memory(&recvd_data)
            .map_err(|_| err_invalid_input())?
            .to_luma8();
        let mut img = rqrr::PreparedImage::prepare(image);
        let grids = img.detect_grids();
        let mut text = String::new();
        for g in grids {
            let (_, content) = g.decode().map_err(|_| err_invalid_input())?;
            if !text.is_empty() {
                text.push('\n');
            }
            text += &content;
        }
        text
    } else {
        String::from_utf8(recvd_data).map_err(|_| err_invalid_input())?
    };
    decode_data(raw_text, options)
}

fn decode_data(raw_text: impl AsRef<str>, options: &KeyStreamOptions) -> Result<()> {
    let raw_text = raw_text.as_ref();

    if !raw_text.starts_with(KEYSTREAM_MAGIC) {
        Err(err_invalid_input())?;
    }

    let input = assemble_text(raw_text)?;

    // decode text
    let data = URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|_| err_invalid_input())?;
    let msg_data = if let Some(key) = &options.key {
        let nonce_len = (data[0] & 0x1f) as usize;
        let nonce = Nonce::from_slice(&data[1..1 + nonce_len]);
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        cipher
            .decrypt(nonce, &data[1 + nonce_len..])
            .map_err(|_| err_invalid_input())?
    } else {
        data
    };
    std::io::stdout().write_all(&msg_data)?;

    Ok(())
}

fn assemble_text(input: &str) -> Result<String> {
    let mut num_parts = 0;
    let mut part_list = Vec::<(u8, &str)>::new();
    for line in input.lines() {
        // magic check
        if !line.starts_with(KEYSTREAM_MAGIC) {
            continue; // we skip lines that don't start with the magic
        }
        let mut colpos = line.find(';').ok_or_else(err_invalid_input)?;
        // version check
        let version_s = &line[KEYSTREAM_MAGIC.len() + 1..colpos];
        match version_s.parse::<u8>() {
            Ok(version) => {
                if version > CURRENT_VERSION {
                    Err(err_value_validation("unsupported version"))?;
                }
            }
            Err(_) => Err(err_value_validation("invalid version str"))?,
        };
        // parts & text
        let mut data_text: Option<&str> = None;
        let mut this_part = 0;
        while colpos < line.len() - 1 {
            let section_start = colpos + 1;
            let next_colpos = line[section_start..]
                .find(';')
                .map(|x| section_start + x)
                .unwrap_or_else(|| line.len());
            let section = &line[section_start..next_colpos];
            if let Some(section) = section.strip_prefix("t=") {
                data_text = Some(section);
            } else if let Some(section) = section.strip_prefix("p=") {
                let p_u8 = u8::from_str_radix(section, 16)
                    .map_err(|_| err_value_validation("invalid part information"))?;
                this_part = (p_u8 >> 4) - 1;
                let total_parts = p_u8 & 0x0f;
                if num_parts == 0 {
                    num_parts = total_parts as usize;
                } else if num_parts != total_parts as usize {
                    Err(err_value_validation("inconsistent number of parts"))?;
                }
            }
            colpos = next_colpos;
        }
        part_list.push((this_part, data_text.ok_or_else(err_invalid_input)?));
    }

    // sanity check
    if part_list.is_empty() {
        Err(err_invalid_input())?;
    }

    // sort parts
    part_list.sort_by(|a, b| a.0.cmp(&b.0));

    // combine parts and return
    let mut combined_text = String::new();
    for i in 0..num_parts {
        match part_list.get(i) {
            Some((idx, txt)) => {
                if *idx != i as u8 {
                    Err(err_invalid_input())?;
                }
                combined_text += txt;
            }
            None => Err(err_value_validation("incomplete list of input"))?,
        }
    }
    Ok(combined_text)
}
