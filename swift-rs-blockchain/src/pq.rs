//! Hybrid signatures: ed25519 plus ML-DSA-44 (FIPS 204).
//!
//! A hybrid signature is valid only when both halves verify. ML-DSA is the
//! post-quantum half. Ed25519 remains so one algorithm failing open does not
//! accept the signature by itself.

use fips204::ml_dsa_44::{self, PrivateKey, PublicKey, SIG_LEN};
use fips204::traits::{KeyGen, Signer, Verifier};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

use crate::keys::{self, KeyPair};

pub struct MlDsaKey {
    pub public_key: PublicKey,
    secret_key: PrivateKey,
}

impl MlDsaKey {
    pub fn generate(seed: u64) -> Result<Self, &'static str> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let (public_key, secret_key) = ml_dsa_44::KG::try_keygen_with_rng(&mut rng)?;
        Ok(Self {
            public_key,
            secret_key,
        })
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, &'static str> {
        let signature = self.secret_key.try_sign(message, &[])?;
        Ok(signature.to_vec())
    }
}

pub struct HybridSignature {
    pub classical: [u8; 64],
    pub ml_dsa: [u8; SIG_LEN],
}

pub fn sign_hybrid(
    classical: &KeyPair,
    ml_dsa: &MlDsaKey,
    message: &[u8],
) -> Result<HybridSignature, &'static str> {
    let raw = ml_dsa.sign(message)?;
    let mut pq = [0u8; SIG_LEN];
    pq.copy_from_slice(&raw);
    Ok(HybridSignature {
        classical: classical.sign(message),
        ml_dsa: pq,
    })
}

pub fn verify_hybrid(
    classical: &ed25519_dalek::VerifyingKey,
    ml_dsa: &PublicKey,
    message: &[u8],
    signature: &HybridSignature,
) -> bool {
    keys::verify(classical, message, &signature.classical)
        && ml_dsa.verify(message, &signature.ml_dsa, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_signature_requires_both_halves() {
        let classical = KeyPair::from_seed(b"pq-classical");
        let ml_dsa = MlDsaKey::generate(7).unwrap();
        let message = b"SCBPS-VOTE-v1";
        let mut signature = sign_hybrid(&classical, &ml_dsa, message).unwrap();
        assert!(verify_hybrid(
            &classical.verifying,
            &ml_dsa.public_key,
            message,
            &signature
        ));
        signature.classical[0] ^= 0xff;
        assert!(!verify_hybrid(
            &classical.verifying,
            &ml_dsa.public_key,
            message,
            &signature
        ));
        signature.classical[0] ^= 0xff;
        signature.ml_dsa[0] ^= 0xff;
        assert!(!verify_hybrid(
            &classical.verifying,
            &ml_dsa.public_key,
            message,
            &signature
        ));
    }
}
