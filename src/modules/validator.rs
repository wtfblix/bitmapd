use serde_json::Value;

#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

pub struct Validator;

impl Validator {
    /// Validates a District claim against block availability
    pub fn validate_district(number: u64, current_tip: u64) -> ValidationResult {
        if number > current_tip {
            return ValidationResult::Invalid(format!(
                "District {} references a future block (Tip: {})",
                number, current_tip
            ));
        }
        ValidationResult::Valid
    }

    /// Validates a Parcel claim against block height and transaction count
    pub fn validate_parcel(
        tx_index: u64, 
        block_number: u64, 
        current_tip: u64, 
        block_data: &Value
    ) -> ValidationResult {
        // 1. Check if block exists
        if block_number > current_tip {
            return ValidationResult::Invalid(format!("Parcel references future block {}", block_number));
        }

        // 2. Check if tx_index is valid for that block
        if let Some(target_block_txs) = block_data["transactions"].as_array() {
            if tx_index >= target_block_txs.len() as u64 {
                return ValidationResult::Invalid(format!(
                    "TX Index {} out of range for block {} (Max: {})",
                    tx_index, block_number, target_block_txs.len() - 1
                ));
            }
        } else {
            return ValidationResult::Invalid(format!("Could not verify transactions for block {}", block_number));
        }

        ValidationResult::Valid
    }
}