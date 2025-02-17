use std::{
    fmt::Display,
    fs,
    io::{self, BufReader, Read, Seek},
    ops::{Div, Rem},
    path::Path,
};

#[derive(Debug)]
pub enum SpirvReadError {
    IoError(io::Error),
    InvalidSpirvFile,
}

impl From<io::Error> for SpirvReadError {
    fn from(value: io::Error) -> Self {
        SpirvReadError::IoError(value)
    }
}

impl Display for SpirvReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpirvReadError::IoError(error) => write!(f, "io error: {error}"),
            SpirvReadError::InvalidSpirvFile => write!(f, "invalid SPIR-V file"),
        }
    }
}

pub fn read_spirv(path: &Path) -> Result<Vec<u32>, SpirvReadError> {
    let mut file = fs::File::open(path)?;

    // Get the size of file - need two seek ops for this.
    let size = file.seek(io::SeekFrom::End(0))?;
    file.rewind()?;

    let data_len = size.div(4);

    // We expect a file of 4 byte words for SPIR-V.
    let remainder = size.rem(4);
    if remainder != 0 {
        return Err(SpirvReadError::InvalidSpirvFile);
    }

    let mut reader = BufReader::new(file);

    let mut data: Vec<u32> = Vec::with_capacity(data_len as usize);
    let mut bytes: [u8; 4] = [0, 0, 0, 0];

    loop {
        match reader.read_exact(&mut bytes) {
            Ok(_) => {
                data.push(u32::from_le_bytes(bytes));
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                } else {
                    return Err(err.into());
                }
            }
        }
    }

    return Ok(data);
}
