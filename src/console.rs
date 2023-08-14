//! Console related functionality. Functions defined here act specifically on `/dev/tty`.

use super::error::Result;
use std::fs::File;
use std::io::{BufRead, Write};
use std::os::fd::{AsFd, AsRawFd};

/// Prints a message to the current terminal, if available.
pub(crate) fn println(message: impl AsRef<str>) {
    if let Ok(mut file) = File::options().write(true).open("/dev/tty") {
        _ = file.write_all(message.as_ref().as_bytes());
        _ = file.write(&[b'\n']);
    }
}

/// Prompts the user for input. If `confidential` is set to `true`, the input is read
/// as a password.
pub(crate) fn prompt(message: impl AsRef<str>, confidential: bool) -> Result<String> {
    let mut file = File::options().read(true).write(true).open("/dev/tty")?;
    file.write_all(message.as_ref().as_bytes())?;
    _ = file.write(&[b':', b' '])?;
    file.flush()?;

    let fd = file.as_fd();
    let fd_raw: i32 = fd.as_raw_fd();
    let mut old_attribs: libc::termios = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    if confidential {
        if unsafe { libc::tcgetattr(fd_raw, &mut old_attribs) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut new_attribs = old_attribs;
        new_attribs.c_lflag &= !libc::ECHO;
        new_attribs.c_lflag |= libc::ECHONL;
        if unsafe { libc::tcsetattr(fd_raw, libc::TCSANOW, &new_attribs) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    let mut input = String::new();
    let mut reader = std::io::BufReader::new(&mut file);
    reader.read_line(&mut input)?;
    if confidential && unsafe { libc::tcsetattr(fd_raw, libc::TCSANOW, &old_attribs) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(input.trim().to_string())
}
