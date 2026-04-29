#[derive(Debug, PartialEq)]
pub enum BitmapClaim {
    District { 
        number: u64 
    },
    Parcel { 
        tx_index: u64, 
        block_number: u64 
    },
    Invalid,
}

pub struct Parser;

impl Parser {
    pub fn parse(content: &str) -> BitmapClaim {
        let lowered = content.to_lowercase();
        let clean = lowered.trim();
        
        if !clean.ends_with(".bitmap") {
            return BitmapClaim::Invalid;
        }

        let parts: Vec<&str> = clean.trim_end_matches(".bitmap").split('.').collect();

        match parts.len() {
            1 => {
                // District: <number>.bitmap
                if let Ok(num) = parts[0].parse::<u64>() {
                    BitmapClaim::District { number: num }
                } else {
                    BitmapClaim::Invalid
                }
            },
            2 => {
                // Parcel: <tx_index>.<block>.bitmap
                if let (Ok(tx), Ok(blk)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    BitmapClaim::Parcel { tx_index: tx, block_number: blk }
                } else {
                    BitmapClaim::Invalid
                }
            },
            _ => BitmapClaim::Invalid,
        }
    }
}