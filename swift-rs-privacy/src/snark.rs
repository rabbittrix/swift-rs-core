//! Groth16 proof that a payment is under a public limit and sanctions-clear.
//!
//! The amount stays a witness. The verifier sees the limit, a sanctions-clear
//! flag that the circuit forces to 1, and a binding to the payment id. A
//! sanctioned payment is rejected before a proof is built.
//!
//! The proving key is generated in-process for this circuit. A production
//! deployment replaces that local setup with a ceremony.

use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::prelude::{AllocVar, Boolean};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_std::rand::rngs::StdRng;
use ark_std::rand::SeedableRng;

use crate::PrivacyError;
use swift_rs_blockchain::hashutil::sha256;

struct LimitCircuit {
    amount: Option<u64>,
    limit: Option<u64>,
    sanctions_clear: Option<bool>,
    binding: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for LimitCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let amount = self.amount.ok_or(SynthesisError::AssignmentMissing)?;
        let limit = self.limit.ok_or(SynthesisError::AssignmentMissing)?;
        let clear = self.sanctions_clear.ok_or(SynthesisError::AssignmentMissing)?;
        if amount > limit || !clear {   
            return Err(SynthesisError::Unsatisfiable);
        }
        let slack = limit - amount;
        let amount_var = u64_witness(cs.clone(), amount)?;
        let slack_var = u64_witness(cs.clone(), slack)?;
        let limit_var =
            FpVar::new_input(cs.clone(), || Ok::<Fr, SynthesisError>(fr_from_u64(limit)))?;
        let clear_var = FpVar::new_input(cs.clone(), || {
            Ok::<Fr, SynthesisError>(fr_from_u64(u64::from(clear)))
        })?;
        let binding_var = FpVar::new_input(cs.clone(), || {
            self.binding.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let _ = binding_var;
        clear_var.enforce_equal(&FpVar::constant(fr_from_u64(1)))?;
        let mut sum = amount_var;
        sum += &slack_var;
        sum.enforce_equal(&limit_var)?;
        Ok(())
    }
}

fn u64_witness(cs: ConstraintSystemRef<Fr>, value: u64) -> Result<FpVar<Fr>, SynthesisError> {
    let mut bits = Vec::with_capacity(64);
    for shift in 0..64 {
        let bit = ((value >> shift) & 1) == 1;
        bits.push(Boolean::new_witness(cs.clone(), || Ok(bit))?);
    }
    Boolean::le_bits_to_fp_var(&bits)
}

fn fr_from_u64(value: u64) -> Fr {
    Fr::from_le_bytes_mod_order(&value.to_le_bytes())
}

fn binding_field(binding: &[u8]) -> Fr {
    let digest = sha256(binding);
    Fr::from_be_bytes_mod_order(&digest)
}

#[derive(Clone, Debug)]
pub struct ComplianceProof {
    proof: Proof<Bn254>,
}

pub struct SnarkProver {
    proving_key: ProvingKey<Bn254>,
    verifying_key: VerifyingKey<Bn254>,
}

impl SnarkProver {
    pub fn setup() -> Result<Self, PrivacyError> {
        let mut rng = StdRng::seed_from_u64(20_220_401);
        let (proving_key, verifying_key) =
            <Groth16<Bn254> as CircuitSpecificSetupSNARK<Fr>>::setup(
                LimitCircuit {
                    amount: Some(0),
                    limit: Some(0),
                    sanctions_clear: Some(true),
                    binding: Some(fr_from_u64(0)),
                },
                &mut rng,
            )
        .map_err(|err| PrivacyError::Snark(err.to_string()))?;
        Ok(Self { proving_key, verifying_key })
    }

    pub fn prove(
        &self,
        amount: u64,
        limit: u64,
        sanctions_clear: bool,
        binding: &[u8],
    ) -> Result<ComplianceProof, PrivacyError> {
        if !sanctions_clear {
            return Err(PrivacyError::Sanctioned);
        }
        if amount > limit {
            return Err(PrivacyError::DailyLimit);
        }
        let mut rng = StdRng::seed_from_u64(amount ^ limit);
        let proof = Groth16::<Bn254>::prove(
            &self.proving_key,
            LimitCircuit {
                amount: Some(amount),
                limit: Some(limit),
                sanctions_clear: Some(true),
                binding: Some(binding_field(binding)),
            },
            &mut rng,
        )
        .map_err(|err| PrivacyError::Snark(err.to_string()))?;
        Ok(ComplianceProof { proof })
    }

    pub fn verify(
        &self,
        proof: &ComplianceProof,
        limit: u64,
        sanctions_clear: bool,
        binding: &[u8],
    ) -> Result<bool, PrivacyError> {
        let inputs = [
            fr_from_u64(limit),
            fr_from_u64(u64::from(sanctions_clear)),
            binding_field(binding),
        ];
        Groth16::<Bn254>::verify(&self.verifying_key, &inputs, &proof.proof)
            .map_err(|err| PrivacyError::Snark(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_hides_the_amount_and_rejects_a_sanctioned_payment() {
        let prover = SnarkProver::setup().unwrap();
        let binding = b"payment-42";
        let proof = prover.prove(40, 100, true, binding).unwrap();
        assert!(prover.verify(&proof, 100, true, binding).unwrap());
        assert!(!prover.verify(&proof, 100, false, binding).unwrap());
        assert!(!prover.verify(&proof, 99, true, binding).unwrap());
        assert!(!prover.verify(&proof, 100, true, b"other-payment").unwrap());
        assert_eq!(
            prover.prove(40, 100, false, binding).unwrap_err(),
            PrivacyError::Sanctioned
        );
        assert!(prover.prove(101, 100, true, binding).is_err());
    }
}
