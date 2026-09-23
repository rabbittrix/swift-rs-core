use sha2::{Digest, Sha256};
use sha3::Sha3_256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
    Sha3_256,
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha3_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hasher.finalize().into()
}

impl HashAlgo {
    pub fn digest(self, data: &[u8]) -> [u8; 32] {
        match self {
            Self::Sha256 => sha256(data),
            Self::Sha3_256 => sha3_256(data),
        }
    }
}

pub fn push_len_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), &'static str> {
    let len = u16::try_from(bytes.len()).map_err(|_| "field too long")?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}
