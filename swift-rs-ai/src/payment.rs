//! Transfer risk score used as a second check after compliance screening.

use crate::fraud::{FraudScore, RiskLevel};

#[derive(Clone, Debug)]
pub struct TransferFeatures {
    pub amount: u128,
    pub daily_spent_before: u128,
    pub daily_limit: u128,
    pub purpose_allowed: bool,
    pub sanctioned: bool,
    pub new_counterparty: bool,
    pub equal_split_count: u32,
}

pub fn score_transfer(features: &TransferFeatures) -> FraudScore {
    let mut flags = Vec::new();
    let mut score: f64 = 0.05;
    if features.sanctioned {
        score = 1.0;
        flags.push("sanctions".into());
    }
    if !features.purpose_allowed {
        score = score.max(0.95);
        flags.push("purpose".into());
    }
    if features
        .daily_spent_before
        .saturating_add(features.amount)
        > features.daily_limit
    {
        score = score.max(0.9);
        flags.push("daily-limit".into());
    }
    if features.new_counterparty && features.amount > features.daily_limit / 2 {
        score = score.max(0.55);
        flags.push("new-counterparty".into());
    }
    if features.equal_split_count >= 5 {
        score = score.max(0.6);
        flags.push("structured-amounts".into());
    }
    let risk_level = if score >= 0.85 {
        RiskLevel::Critical
    } else if score >= 0.5 {
        RiskLevel::High
    } else if score >= 0.3 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    FraudScore {
        score,
        risk_level,
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanctions_and_structuring_raise_the_score() {
        let critical = score_transfer(&TransferFeatures {
            amount: 1,
            daily_spent_before: 0,
            daily_limit: 100,
            purpose_allowed: true,
            sanctioned: true,
            new_counterparty: false,
            equal_split_count: 0,
        });
        assert_eq!(critical.risk_level, RiskLevel::Critical);
        let structured = score_transfer(&TransferFeatures {
            amount: 10,
            daily_spent_before: 0,
            daily_limit: 1_000,
            purpose_allowed: true,
            sanctioned: false,
            new_counterparty: false,
            equal_split_count: 5,
        });
        assert_eq!(structured.risk_level, RiskLevel::High);
    }
}
