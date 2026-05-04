use serde_json::Value;

#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

pub struct Validator;

impl Validator {
    /// Validates a District claim against the block currently being scanned
    pub fn validate_district(number: u64, scan_block: u64) -> ValidationResult {
        if number > scan_block {
            return ValidationResult::Invalid(format!(
                "District {} references a future block (Scan block: {})",
                number, scan_block
            ));
        }
        ValidationResult::Valid
    }

    /// Validates a Parcel claim against the block currently being scanned
    pub fn validate_parcel(
        tx_index: u64,
        block_number: u64,
        scan_block: u64,
        target_block_data: &Value
    ) -> ValidationResult {
        // 1. Check if the claimed block existed at scan time
        if block_number > scan_block {
            return ValidationResult::Invalid(format!(
                "Parcel references future block {} (Scan block: {})",
                block_number, scan_block
            ));
        }

        // 2. Check if tx_index is valid for that target block
        // Note: target_block_data must be the data for 'block_number', not 'scan_block'
        if let Some(transactions) = target_block_data["transactions"].as_array() {
            if tx_index >= transactions.len() as u64 {
                return ValidationResult::Invalid(format!(
                    "TX Index {} out of range for block {} (Max: {})",
                    tx_index, block_number, transactions.len().saturating_sub(1)
                ));
            }
        } else {
            return ValidationResult::Invalid(format!(
                "Required block data for {} is missing transactions",
                block_number
            ));
        }

        ValidationResult::Valid
    }
}