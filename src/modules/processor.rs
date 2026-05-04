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
                if let Ok(target_block_data) = self.client.get_block(block_number).await {
                    match Validator::validate_parcel(tx_index, block_number, scan_block, &target_block_data) {
                        ValidationResult::Valid => {

                            // 🔥 NEW: fetch inscription metadata
                            let inscription = match self.client.get_inscription(id).await {
                                Ok(data) => data,
                                Err(_) => {
                                    println!("❌ PARCEL INVALID: failed to fetch inscription metadata");
                                    continue;
                                }
                            };

                            // 🔥 NEW: extract parents array
                            let parents = match inscription["parents"].as_array() {
                                Some(p) if !p.is_empty() => p,
                                _ => {
                                    println!("❌ PARCEL INVALID: no parent");
                                    continue;
                                }
                            };

                            // 🔥 NEW: validate parent against known districts
                            let mut valid_parent: Option<u64> = None;

                            for parent in parents {
                                if let Some(parent_id) = parent.as_str() {
                                    if let Some(district_number) = self.db.get_district_by_inscription(parent_id)? {
                                        valid_parent = Some(district_number);
                                        break;
                                    }
                                }
                            }

                            let parent_district = match valid_parent {
                                Some(d) => d,
                                None => {
                                    println!("❌ PARCEL INVALID: parent is not a valid district");
                                    continue;
                                }
                            };

                            // ✅ ONLY VALID NOW
                            if self.db.save_parcel(tx_index, block_number, id, parent_district)? {
                                println!(
                                    "✅ PARCEL ACCEPTED: {}.{}.bitmap | parent: {} | ID: {}",
                                    tx_index, block_number, parent_district, id
                                );
                            } else {
                                println!(
                                    "⚠️ PARCEL REJECTED: {}.{}.bitmap (Duplicate) | ID: {}",
                                    tx_index, block_number, id
                                );
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