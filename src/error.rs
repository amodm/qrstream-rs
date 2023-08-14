#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Arg(clap::Error),
    Crypto(aes_gcm::Error),
    Qr(qr_code::types::QrError),
}

impl Error {
    pub(crate) fn exit(&self) -> ! {
        if let Error::Arg(clap_err) = &self {
            clap_err.exit();
        } else {
            eprintln!("Error: {:?}", &self);
            std::process::exit(1);
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<clap::Error> for Error {
    fn from(e: clap::Error) -> Self {
        Error::Arg(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl From<aes_gcm::Error> for Error {
    fn from(e: aes_gcm::Error) -> Self {
        Error::Crypto(e)
    }
}

impl From<qr_code::types::QrError> for Error {
    fn from(e: qr_code::types::QrError) -> Self {
        Error::Qr(e)
    }
}

pub(crate) fn usage_err(message: impl AsRef<str>) -> ! {
    clap::Command::new(clap::crate_name!())
        .error(clap::error::ErrorKind::ValueValidation, message.as_ref())
        .exit()
}

pub(crate) fn err_value_validation(message: impl AsRef<str>) -> clap::error::Error {
    clap::Command::new(clap::crate_name!())
        .error(clap::error::ErrorKind::ValueValidation, message.as_ref())
}

pub(crate) fn err_invalid_input() -> clap::error::Error {
    err_value_validation("invalid input")
}

pub(crate) fn io_error(message: impl AsRef<str>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, message.as_ref())
}

pub(crate) trait UnwrapOrExit<T> {
    /// Return the wrapped value, or exit the process with an error.
    fn unwrap_or_exit(self) -> T;
}

impl<T> UnwrapOrExit<T> for Result<T> {
    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => error.exit(),
        }
    }
}
