use wasmi::{Config, Engine, Linker, Module, Store};

use crate::error::ChainError;

pub const CBDC_LIMIT: &str = include_str!("../contracts/cbdc_limit.wat");
pub const SWAP_GUARD: &str = include_str!("../contracts/swap_guard.wat");
pub const GOVERNANCE_QUORUM: &str = include_str!("../contracts/governance_quorum.wat");
pub const RWA_WHITELIST: &str = include_str!("../contracts/rwa_whitelist.wat");

pub struct WasmPolicy {
    engine: Engine,
    fuel: u64,
}

impl WasmPolicy {
    pub fn new(fuel: u64) -> Result<Self, ChainError> {
        let mut config = Config::default();
        config.consume_fuel(true);
        Ok(Self {
            engine: Engine::new(&config),
            fuel,
        })
    }

    pub fn call_i64_pair(&self, wat_source: &str, export: &str, left: i64, right: i64) -> Result<i32, ChainError> {
        let wasm = wat::parse_str(wat_source).map_err(|e| ChainError::Wasm(e.to_string()))?;
        let module = Module::new(&self.engine, wasm.as_slice())
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(self.fuel)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| ChainError::Wasm(e.to_string()))?
            .start(&mut store)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let func = instance
            .get_typed_func::<(i64, i64), i32>(&store, export)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        func.call(&mut store, (left, right))
            .map_err(|e| ChainError::Wasm(e.to_string()))
    }

    pub fn call_i32(&self, wat_source: &str, export: &str, flag: i32) -> Result<i32, ChainError> {
        let wasm = wat::parse_str(wat_source).map_err(|e| ChainError::Wasm(e.to_string()))?;
        let module = Module::new(&self.engine, wasm.as_slice())
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(self.fuel)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| ChainError::Wasm(e.to_string()))?
            .start(&mut store)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        let func = instance
            .get_typed_func::<i32, i32>(&store, export)
            .map_err(|e| ChainError::Wasm(e.to_string()))?;
        func.call(&mut store, flag)
            .map_err(|e| ChainError::Wasm(e.to_string()))
    }
}

/// Programmable spending rule. Amounts above `u64::MAX` are rejected before WASM.
pub fn within_limit(amount: u128, limit: u128) -> Result<bool, ChainError> {
    let amount = i64::try_from(amount).map_err(|_| ChainError::Wasm("amount exceeds wasm word".into()))?;
    let limit = i64::try_from(limit).map_err(|_| ChainError::Wasm("limit exceeds wasm word".into()))?;
    let policy = WasmPolicy::new(100_000)?;
    Ok(policy.call_i64_pair(CBDC_LIMIT, "within_limit", amount, limit)? == 1)
}

pub fn quorum_met(votes: u128, total: u128) -> Result<bool, ChainError> {
    let votes = i64::try_from(votes).map_err(|_| ChainError::Wasm("votes exceed wasm word".into()))?;
    let total = i64::try_from(total).map_err(|_| ChainError::Wasm("total exceeds wasm word".into()))?;
    let policy = WasmPolicy::new(100_000)?;
    Ok(policy.call_i64_pair(GOVERNANCE_QUORUM, "quorum_met", votes, total)? == 1)
}

pub fn whitelist_allowed(flag: bool) -> Result<bool, ChainError> {
    let policy = WasmPolicy::new(100_000)?;
    Ok(policy.call_i32(RWA_WHITELIST, "allowed", i32::from(flag))? == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbdc_limit_and_quorum_contracts() {
        assert!(within_limit(10, 10).unwrap());
        assert!(!within_limit(11, 10).unwrap());
        assert!(quorum_met(67, 100).unwrap());
        assert!(!quorum_met(66, 100).unwrap());
        assert!(whitelist_allowed(true).unwrap());
        assert!(!whitelist_allowed(false).unwrap());
    }

    #[test]
    fn fuel_stops_a_non_terminating_module() {
        let spinning = r#"
            (module
              (func (export "within_limit") (param i64) (param i64) (result i32)
                (loop $forever (br $forever))
                (i32.const 0)))
        "#;
        let policy = WasmPolicy::new(1_000).unwrap();
        let err = policy
            .call_i64_pair(spinning, "within_limit", 1, 1)
            .unwrap_err();
        assert!(matches!(err, ChainError::Wasm(_)));
    }
}
