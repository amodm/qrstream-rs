# qrstream

[![Current Version](https://img.shields.io/crates/v/qrstream.svg)](https://crates.io/crates/qrstream)
[![License](https://img.shields.io/crates/l/qrstream.svg)](#license)
![Build](https://github.com/amodm/qrstream-rs/workflows/Build/badge.svg?branch=main)

A secure no-persistence way to convert confidential data into encrypted QR codes, and vice-versa. If the secret doesn't fit into a single QR code, it is automatically split into multiple QR codes (the decoding process knows how to reassemble them later).

## Encode to QR

#### Basic use
`echo "MYSECRET" | qrstream -p prompt encode > my-secret-qr.png`

#### Print QR without storing to disk
`echo "MYSECRET" | qrstream -p prompt encode | lpr`

## Decode

#### From camera (requires connected webcam)
`echo "MYSECRET" | qrstream -p prompt -i camera decode > outfile`

#### From stdin
`cat my-secret-qr.png | qrstream -p prompt decode > outfile`

## License

`SPDX-License-Identifier: Apache-2.0 OR MIT`

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.