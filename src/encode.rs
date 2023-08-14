use aes_gcm::{
    aead::{
        rand_core::{OsRng, RngCore},
        Aead, AeadCore, KeyInit,
    },
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use image::{GenericImage, GrayImage, ImageBuffer, Luma};
use qr_code::{EcLevel, QrCode};
use std::io::{Cursor, Write};

use crate::{
    error::{io_error, usage_err},
    OutputFormat, CURRENT_VERSION, KEYSTREAM_MAGIC,
};

use super::error::{Error, Result};
use super::KeyStreamOptions;

pub(crate) async fn encode(options: &KeyStreamOptions) -> Result<()> {
    let input = options.input.get_content().await?;
    let result_list = encode_data(&input, options)?;

    let encode_options = options.encode_options();
    match encode_options.out_format {
        OutputFormat::Txt => {
            for (output, _) in &result_list {
                println!("{}", output);
            }
        }
        OutputFormat::Png => write_as_png(&result_list, encode_options.qr_per_row)?,
    }

    Ok(())
}

fn encode_data(u8_data: &[u8], options: &KeyStreamOptions) -> Result<Vec<(String, QrCode)>> {
    let bin_data = if let Some(key) = &options.key {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut enc_data = Vec::<u8>::new();
        let nonce_len = ((OsRng.next_u32() & 0xe0) as u8) | (nonce.len() as u8 & 0x1f);
        enc_data.push(nonce_len);
        enc_data.extend_from_slice(nonce.as_slice());
        enc_data.extend(cipher.encrypt(&nonce, u8_data)?);
        enc_data
    } else {
        u8_data.to_vec()
    };
    let data = URL_SAFE_NO_PAD.encode(bin_data);

    let ec_level = options.encode_options().ec_level;
    let mut parts_needed = 1;
    while parts_needed < 16 {
        let part_len = (data.len() + parts_needed - 1) / parts_needed;
        let first_part = &data[..part_len];
        match encode_to_qr(first_part, 0, parts_needed as u8, ec_level) {
            Ok((output, qr)) => {
                let mut result_list = Vec::<(String, QrCode)>::new();
                result_list.push((output, qr));
                for current_part in 1..parts_needed {
                    let idx1 = current_part * part_len;
                    let idx2 = std::cmp::min((current_part + 1) * part_len, data.len());
                    let part = &data[idx1..idx2];
                    let (output, qr) =
                        encode_to_qr(part, current_part as u8, parts_needed as u8, ec_level)?;
                    result_list.push((output, qr));
                }
                return Ok(result_list);
            }
            Err(Error::Qr(qr_code::types::QrError::DataTooLong)) => {
                parts_needed += 1;
                continue;
            }
            Err(err) => {
                return Err(err);
            }
        };
    }
    usage_err("data too large to encode")
}

fn encode_to_qr(data: &str, part: u8, total: u8, level: EcLevel) -> Result<(String, QrCode)> {
    let mut output = String::new();
    output += format!("{}/{};", KEYSTREAM_MAGIC, CURRENT_VERSION).as_str();
    output += format!("p={:x};", ((part + 1) << 4) | (total & 0x0f)).as_str();
    output += "t=";
    output += data;

    let output_bytes = output.as_bytes();
    let qr = QrCode::with_error_correction_level(output_bytes, level)?;
    Ok((output, qr))
}

fn write_as_png(qr_vec: &[(String, QrCode)], codes_per_row: u32) -> Result<()> {
    let spacing = 64;
    let mut img_vec = Vec::<ImageBuffer<Luma<u8>, Vec<u8>>>::new();
    let mut img_width = 0;
    let mut img_height = spacing;
    for (idx, (_, qr)) in qr_vec.iter().enumerate() {
        let qr_size = qr.width() as u32;
        let pixel_per_mod = std::cmp::max(4, (360f32 / qr_size as f32).floor() as u32); // technically, sqrt
        let qr_img_size = qr_size * pixel_per_mod;
        img_width = std::cmp::max(
            img_width,
            qr_img_size * codes_per_row + spacing * (codes_per_row + 1),
        );
        if idx % codes_per_row as usize == 0 {
            img_height += qr_img_size + spacing;
        }
        let mut img = GrayImage::new(qr_img_size, qr_img_size);
        qr.to_vec()
            .chunks(qr_size as usize)
            .enumerate()
            .for_each(|(y, row)| {
                row.iter().enumerate().for_each(|(x, val)| {
                    let val = if *val { 0 } else { 255 };
                    for dy in 0..pixel_per_mod {
                        for dx in 0..pixel_per_mod {
                            img.put_pixel(
                                x as u32 * pixel_per_mod + dx,
                                y as u32 * pixel_per_mod + dy,
                                image::Luma([val]),
                            );
                        }
                    }
                });
            });
        img_vec.push(img);
    }
    let mut img = GrayImage::new(img_width, img_height);
    img.fill(255);
    let mut x = spacing;
    let mut y = spacing;
    for (idx, qr_img) in img_vec.iter().enumerate() {
        img.copy_from(qr_img, x, y).unwrap();
        if idx % codes_per_row as usize != codes_per_row as usize - 1 {
            x += qr_img.width() + spacing;
        } else {
            x = spacing;
            y += qr_img.height() + spacing;
        }
    }
    let mut png_data = Vec::<u8>::new();
    let mut writer = Cursor::new(&mut png_data);
    image::write_buffer_with_format(
        &mut writer,
        &img,
        img.width(),
        img.height(),
        image::ColorType::L8,
        image::ImageFormat::Png,
    )
    .map_err(|e| io_error(e.to_string()))?;
    std::io::stdout().write_all(&png_data)?;
    Ok(())
}
