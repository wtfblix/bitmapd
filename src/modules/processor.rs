use anyhow::Result;
use crate::modules::ord_client::OrdClient;
use crate::modules::parser::{Parser, BitmapClaim};
use crate::modules::validator::{Validator, ValidationResult};
use crate::modules::database::Database;

pub struct BlockProcessor<'a> {
    client: &'a OrdClient,
    db: &'a Database,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(client: &'a OrdClient, db: &'a Database) -> Self {
        Self { client, db }
    }

    pub async fn process_block(&self, height: u64, current_tip: u64) -> Result<()> {
        println!("--- Processing Block {} ---", height);

        // 1. Fetch Block Data
        let block_data = self.client.get_block(height).await?;
        
        let inscriptions = match block_data["inscriptions"].as_array() {
            Some(i) => i,
            None => {
                println!("No inscriptions in block {}. Skipping.", height);
                return Ok(());
            }
        };

        println!("Found {} inscriptions to scan...", inscriptions.len());

        // 2. Iterate through inscriptions in order
        for id_val in inscriptions {
            if let Some(id) = id_val.as_str() {
                // 3. Fetch content
                if let Ok(raw_content) = self.client.get_content(id).await {
                    // 4. Parse
                    match Parser::parse(&raw_content) {
                        BitmapClaim::District { number } => {
                            // 5. Validate District
                            match Validator::validate_district(number, current_tip) {
                                ValidationResult::Valid => {
                                    // 6. Save to DB (First claim wins logic in DB)
                                    if self.db.save_district(number, id, height)? {
                                        println!("✅ ACCEPTED: {}.bitmap | ID: {}", number, id);
                                    } else {
                                        println!("⚠️ REJECTED: {}.bitmap (Duplicate Claim) | ID: {}", number, id);
                                    }
                                },
                                ValidationResult::Invalid(reason) => {
                                    println!("❌ REJECTED: {}.bitmap (Rule Violation: {})", number, reason);
                                }
                            }
                        },
                        BitmapClaim::Parcel { tx_index, block_number } => {
                            // We will implement Parcel validation + Parent checks in the next iteration
                            // For now, we focus on nailing Districts
                            // println!("📦 Parcel claim found: {}.{}.bitmap", tx_index, block_number);
                        },
                        BitmapClaim::Invalid => {} // Skip noise
                    }
                }
            }
        }

        // 7. Update progress
        self.db.set_last_block(height)?;
        
        Ok(())
    }
}