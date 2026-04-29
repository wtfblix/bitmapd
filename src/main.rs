mod modules;

use clap::{Parser, Subcommand};
use modules::database::Database;
use modules::ord_client::OrdClient;
use modules::processor::BlockProcessor;
use tokio::time::{sleep, Duration};

const GENESIS_BITMAP_BLOCKHEIGHT: u64 = 792435;
const ORD_BASE_URL: &str = "http://127.0.0.1:8080";
const DB_PATH: &str = "resolver.db";

#[derive(Parser)]
#[command(name = "bitmapd")]
#[command(version)]
#[command(about = "Bitmap Resolver Daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the bitmap indexer
    Index,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index => run_indexer().await?,
    }

    Ok(())
}

async fn run_indexer() -> anyhow::Result<()> {
    let client = OrdClient::new(ORD_BASE_URL);
    let db = Database::new(DB_PATH)?;
    let processor = BlockProcessor::new(&client, &db);

    println!("bitmapd indexer started");
    println!("Genesis bitmap blockheight: {}", GENESIS_BITMAP_BLOCKHEIGHT);

    loop {
        let mut current_block = db.get_last_block()?;

        if current_block < GENESIS_BITMAP_BLOCKHEIGHT {
            current_block = GENESIS_BITMAP_BLOCKHEIGHT;
        } else {
            current_block += 1;
        }

        match processor.process_block(current_block, 9_999_999).await {
            Ok(_) => {
                println!("Block {} processed", current_block);
            }
            Err(err) => {
                println!(
                    "Waiting for next block or ord may be behind, block {}, error: {}",
                    current_block, err
                );
                sleep(Duration::from_secs(20)).await;
            }
        }
    }
}