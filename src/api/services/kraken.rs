use std::{ collections::HashMap, fs::{self, File}, path::Path};

use serde::Serialize;
use rmp_serde::Serializer;

use crate::{api::{create_kraken_txs, fetch_assets_pair, fetch_history_kraken, map_asset_pairs, KrakenPairs, Tier}, errors::IoError, structs::{transaction::Transaction, wallet::{Wallet, WalletIdMap}}, utils::file_exists};



pub fn handle_kraken_data(    
    wallet_ids: &mut WalletIdMap,
    wallets: &mut HashMap<String, Wallet>,
    txs: &mut Vec<Transaction>) -> Result<(),IoError>{
    
    let pairs: KrakenPairs = kraken_pairs().unwrap();

    let file_path = ".data/kraken/kraken_data";
    if !file_exists(file_path){
        let response = fetch_history_kraken(Tier::Intermediate).unwrap();
        create_kraken_txs(
            wallet_ids,
            wallets,
            txs,
            response.0,
            response.1,
            response.2,
            pairs.get()
        ).map_err(|e| IoError::new(e.to_string()))?;
        let file = File::create(file_path).expect("Unable to create file");
        let mut writer = Serializer::new(file);
        txs.serialize(&mut writer).map_err(|e| IoError::new(e.to_string()))?;
    }
    return Ok(());
}



pub fn kraken_pairs() -> Result<KrakenPairs,IoError>{
    let file_path = ".data/kraken/kraken_pairs";

    if !file_exists(file_path){
        let asset_pairs_raw = fetch_assets_pair().unwrap().result.unwrap().pairs;
        let asset_pairs  = map_asset_pairs(asset_pairs_raw);
    
        // Create directories if they don't exist
        if let Some(parent) = Path::new(&file_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).expect("Failed to create directories");
            }
        }
        let file = File::create(file_path).expect("Unable to create file");
        let mut writer = Serializer::new(file);
        asset_pairs.serialize(&mut writer).map_err(|e| IoError::new(e.to_string()))?;

        return Ok(asset_pairs);
    } else {
        let file = File::open(file_path).map_err(|e|IoError::new(e.to_string()))?;
        let deserialized_map: KrakenPairs = rmp_serde::from_read(file).map_err(|e|IoError::new(e.to_string()))?;
        return Ok(deserialized_map);
    }

}