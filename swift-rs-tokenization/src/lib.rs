//! Real-world asset tokens and a collateralized stablecoin factory.
//!
//! Compliance data is stored as a commitment. The registry does not keep raw
//! identity documents.

use std::collections::{BTreeMap, BTreeSet};


use swift_rs_blockchain::wasm::whitelist_allowed;
use swift_rs_blockchain::{Address, AssetId, Hash};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
    #[error("unknown asset")]
    UnknownAsset,
    #[error("address is not whitelisted")]
    NotWhitelisted,
    #[error("insufficient collateral")]
    InsufficientCollateral,
    #[error("overflow")]
    Overflow,
    #[error("duplicate asset")]
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Security,
    Stablecoin,
    Commodity,
}

#[derive(Clone, Debug)]
pub struct SecurityToken {
    pub asset: AssetId,
    pub name: String,
    pub issuer: Address,
    pub decimals: u8,
    pub partition: String,
    pub whitelist: BTreeSet<Address>,
    pub total_supply: u128,
}

#[derive(Clone, Debug)]
pub struct Stablecoin {
    pub asset: AssetId,
    pub collateral_asset: AssetId,
    pub ratio_bps: u32,
    pub supply: u128,
    pub collateral: u128,
}

#[derive(Clone, Debug)]
pub struct AssetRecord {
    pub asset: AssetId,
    pub kind: AssetKind,
    pub name: String,
    pub issuer: Address,
    pub kyc_commitment: Hash,
    pub parent: Option<AssetId>,
}

#[derive(Clone, Debug, Default)]
pub struct TokenEngine {
    pub securities: BTreeMap<String, SecurityToken>,
    pub stablecoins: BTreeMap<String, Stablecoin>,
    pub registry: Vec<AssetRecord>,
}

impl TokenEngine {
    pub fn issue_security(&mut self, token: SecurityToken, kyc_commitment: Hash) -> Result<(), TokenError> {
        let key = token.asset.0.clone();
        if self.securities.contains_key(&key) {
            return Err(TokenError::Duplicate);
        }
        self.registry.push(AssetRecord {
            asset: token.asset.clone(),
            kind: AssetKind::Security,
            name: token.name.clone(),
            issuer: token.issuer,
            kyc_commitment,
            parent: None,
        });
        self.securities.insert(key, token);
        Ok(())
    }

    pub fn authorize_security_transfer(
        &self,
        asset: &AssetId,
        from: &Address,
        to: &Address,
    ) -> Result<(), TokenError> {
        let token = self
            .securities
            .get(&asset.0)
            .ok_or(TokenError::UnknownAsset)?;
        let allowed = token.whitelist.contains(from) && token.whitelist.contains(to);
        if !allowed || !whitelist_allowed(allowed).unwrap_or(false) {
            return Err(TokenError::NotWhitelisted);
        }
        Ok(())
    }

    pub fn register_stablecoin(&mut self, coin: Stablecoin, issuer: Address, kyc_commitment: Hash) -> Result<(), TokenError> {
        let key = coin.asset.0.clone();
        if self.stablecoins.contains_key(&key) {
            return Err(TokenError::Duplicate);
        }
        self.registry.push(AssetRecord {
            asset: coin.asset.clone(),
            kind: AssetKind::Stablecoin,
            name: key.clone(),
            issuer,
            kyc_commitment,
            parent: Some(coin.collateral_asset.clone()),
        });
        self.stablecoins.insert(key, coin);
        Ok(())
    }

    pub fn required_collateral(&self, asset: &AssetId, mint_amount: u128) -> Result<u128, TokenError> {
        let coin = self
            .stablecoins
            .get(&asset.0)
            .ok_or(TokenError::UnknownAsset)?;
        if coin.ratio_bps == 0 || mint_amount == 0 {
            return Err(TokenError::InsufficientCollateral);
        }
        let collateral = mint_amount
            .checked_mul(u128::from(coin.ratio_bps))
            .ok_or(TokenError::Overflow)?
            / 10_000;
        if collateral == 0 {
            Err(TokenError::InsufficientCollateral)
        } else {
            Ok(collateral)
        }
    }

    pub fn record_mint(&mut self, asset: &AssetId, mint_amount: u128, collateral: u128) -> Result<(), TokenError> {
        let needed = self.required_collateral(asset, mint_amount)?;
        if collateral < needed {
            return Err(TokenError::InsufficientCollateral);
        }
        let coin = self
            .stablecoins
            .get_mut(&asset.0)
            .ok_or(TokenError::UnknownAsset)?;
        coin.supply = coin.supply.checked_add(mint_amount).ok_or(TokenError::Overflow)?;
        coin.collateral = coin
            .collateral
            .checked_add(collateral)
            .ok_or(TokenError::Overflow)?;
        Ok(())
    }

    pub fn record_burn(&mut self, asset: &AssetId, burn_amount: u128, collateral_out: u128) -> Result<(), TokenError> {
        let coin = self
            .stablecoins
            .get_mut(&asset.0)
            .ok_or(TokenError::UnknownAsset)?;
        if coin.supply < burn_amount || coin.collateral < collateral_out {
            return Err(TokenError::InsufficientCollateral);
        }
        coin.supply -= burn_amount;
        coin.collateral -= collateral_out;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swift_rs_blockchain::KeyPair;

    #[test]
    fn security_transfer_requires_both_parties() {
        let issuer = KeyPair::from_seed(b"issuer");
        let holder = KeyPair::from_seed(b"holder");
        let outsider = KeyPair::from_seed(b"outsider");
        let mut engine = TokenEngine::default();
        let asset = AssetId::new("rwa:gold").unwrap();
        let mut whitelist = BTreeSet::new();
        whitelist.insert(issuer.address);
        whitelist.insert(holder.address);
        engine
            .issue_security(
                SecurityToken {
                    asset: asset.clone(),
                    name: "Vaulted gold".into(),
                    issuer: issuer.address,
                    decimals: 6,
                    partition: "series-a".into(),
                    whitelist,
                    total_supply: 1_000_000,
                },
                Hash::sha256(b"kyc-commitment"),
            )
            .unwrap();
        assert!(engine
            .authorize_security_transfer(&asset, &issuer.address, &holder.address)
            .is_ok());
        assert_eq!(
            engine.authorize_security_transfer(&asset, &issuer.address, &outsider.address),
            Err(TokenError::NotWhitelisted)
        );
        assert_eq!(engine.registry[0].kyc_commitment, Hash::sha256(b"kyc-commitment"));
    }

    #[test]
    fn stablecoin_tracks_collateral_ratio() {
        let issuer = KeyPair::from_seed(b"stable-issuer");
        let mut engine = TokenEngine::default();
        let asset = AssetId::new("stable:usd").unwrap();
        engine
            .register_stablecoin(
                Stablecoin {
                    asset: asset.clone(),
                    collateral_asset: AssetId::new("fiat:usd").unwrap(),
                    ratio_bps: 15_000,
                    supply: 0,
                    collateral: 0,
                },
                issuer.address,
                Hash::sha256(b"stable-kyc"),
            )
            .unwrap();
        assert_eq!(engine.required_collateral(&asset, 100).unwrap(), 150);
        assert!(engine.record_mint(&asset, 100, 149).is_err());
        engine.record_mint(&asset, 100, 150).unwrap();
        engine.record_burn(&asset, 40, 60).unwrap();
        assert_eq!(engine.stablecoins["stable:usd"].supply, 60);
        assert_eq!(engine.stablecoins["stable:usd"].collateral, 90);
        assert_eq!(engine.registry[0].parent.as_ref().unwrap().as_str(), "fiat:usd");
    }
}
