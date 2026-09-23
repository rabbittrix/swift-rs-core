use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::hashutil::sha256;
use crate::types::Address;

#[derive(Clone)]
pub struct KeyPair {
    pub signing: SigningKey,
    pub verifying: VerifyingKey,
    pub address: Address,
}

impl KeyPair {
    pub fn from_seed(seed: &[u8]) -> Self {
        let secret = sha256(seed);
        let signing = SigningKey::from_bytes(&secret);
        let verifying = signing.verifying_key();
        let address = address_from_verifying_key(&verifying);
        Self {
            signing,
            verifying,
            address,
        }
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }
}

pub fn address_from_verifying_key(key: &VerifyingKey) -> Address {
    Address(sha256(key.as_bytes()))
}

pub fn verify(key: &VerifyingKey, message: &[u8], signature: &[u8; 64]) -> bool {
    let Ok(signature) = Signature::from_slice(signature) else {
        return false;
    };
    key.verify(message, &signature).is_ok()
}

/// In-process stand-in for an HSM. Production keys stay inside the module.
#[derive(Clone, Default)]
pub struct MemoryKeyStore {
    keys: Vec<KeyPair>,
}

impl MemoryKeyStore {
    pub fn insert(&mut self, key: KeyPair) {
        self.keys.push(key);
    }

    pub fn sign(&self, address: &Address, message: &[u8]) -> Option<[u8; 64]> {
        self.keys
            .iter()
            .find(|key| &key.address == address)
            .map(|key| key.sign(message))
    }

    pub fn get(&self, address: &Address) -> Option<&KeyPair> {
        self.keys.iter().find(|key| &key.address == address)
    }
}
