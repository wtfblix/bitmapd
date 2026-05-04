use anyhow::Result;
use crate::modules::ord_client::OrdClient;
use crate::modules::parser::{Parser, BitmapClaim};
use crate::modules::validator::{Validator, ValidationResult};
use crate::modules::database::Database;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;

pub struct BlockProcessor<'a> {
    client: &'a OrdClient,
    db: &'a Database,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(client: &'a OrdClient, db: &'a Database) -> Self {
        Self { client, db }
    }

    pub async fn process_block(&self, height: u64, scan_block: u64) -> Result<()> {
        println!("--- Processing Block {} ---", height);

        // 1. Fetch current block data to get inscriptions
        let block_data = self.client.get_block(height).await?;
        
        let inscriptions_list = match block_data["inscriptions"].as_array() {
            Some(i) => i.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
            None => {
                println!("No inscriptions in block {}. Skipping.", height);
                self.db.set_last_block(height)?;
                return Ok(());
            }
        };

        if inscriptions_list.is_empty() {
            self.db.set_last_block(height)?;
            return Ok(());
        }

        println!("Found {} inscriptions. Fetching contents in parallel...", inscriptions_list.len());

        // 2. PHASE 1: Parallel Content Fetch (Ordering doesn't matter here)
        let content_map: HashMap<String, String> = stream::iter(inscriptions_list.clone())
            .map(|id| {
                let client = self.client;
                async move {
                    let res = client.get_content(id).await;
                    (id.to_string(), res)
                }
            })
            .buffer_unordered(20) // Concurrent requests to ord
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|(id, res)| res.ok().map(|content| (id, content)))
            .collect();

        // 3. PHASE 2: Sequential Validation (ORDER MATTERS HERE)
        for id in inscriptions_list {
            let raw_content = match content_map.get(id) {
                Some(c) => c,
                None => continue,
            };

            match Parser::parse(raw_content) {
                BitmapClaim::District { number } => {
                    match Validator::validate_district(number, scan_block) {
                        ValidationResult::Valid => {
                            if self.db.save_district(number, id, height)? {
                                println!("✅ DISTRICT ACCEPTED: {}.bitmap | ID: {}", number, id);
                            } else {
                                println!("⚠️ DISTRICT REJECTED: {}.bitmap (Duplicate) | ID: {}", number, id);
                            }
                        },
                        ValidationResult::Invalid(reason) => {
                            println!("❌ DISTRICT INVALID: {}.bitmap ({})", number, reason);
                        }
                    }
                },
                BitmapClaim::Parcel { tx_index, block_number } => {
                    // Fetch the target block data to verify the tx_index
                    // Note: This is an extra call but required for parcel validation rules
                    if let Ok(target_block_data) = self.client.get_block(block_number).await {
                        match Validator::validate_parcel(tx_index, block_number, scan_block, &target_block_data) {
                            ValidationResult::Valid => {
                                if self.db.save_parcel(tx_index, block_number, id, block_number)? {
                                    println!("✅ PARCEL ACCEPTED: {}.{}.bitmap | ID: {}", tx_index, block_number, id);
                                } else {
                                    println!("⚠️ PARCEL REJECTED: {}.{}.bitmap (Duplicate) | ID: {}", tx_index, block_number, id);
                                }
                            },
                            ValidationResult::Invalid(reason) => {
                                println!("❌ PARCEL INVALID: {}.{}.bitmap ({})", tx_index, block_number, reason);
                            }
                        }
                    } else {
                        println!("❌ PARCEL ERROR: Could not fetch block {} to validate parcel", block_number);
                    }
                },
                BitmapClaim::Invalid => {}
            }
        }

        // 4. Update progress
        self.db.set_last_block(height)?;
        Ok(())
    }
}