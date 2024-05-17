use std::fs::File;

use chrono::{DateTime, Utc};
use rmp_serde::Serializer;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{
    api::{
        create_kraken_txs, fetch_assets_pair, fetch_history_kraken, map_asset_pairs, KrakenPairs,
        Tier,
    },
    errors::IoError,
    structs::{transaction::Transaction, wallet_manager::WalletManager},
    utils::{create_directories_if_needed, file_exists},
};

/* Fetch and save the kraken data: Even though we keep the global list of all the transactions in another file (see TransactionsManager),
we still want to keep the specific data of <kraken> (or any exchange) somewhere. It allows us to easily "deactivate/remove" the exchange
or put it again withtout fetching the data again. It also allows for easy update of the data withtout having to handle the full vector of
transactions.
*/
pub fn handle_kraken_data(
    wallet_manager: &mut WalletManager,
) -> Result<Vec<Transaction>, IoError> {
    let mut kraken_txs: Vec<Transaction> = Vec::new();

    let pairs: KrakenPairs = kraken_pairs().unwrap();

    let file_path = ".data/kraken/kraken_data";
    if !file_exists(file_path) {
        let response = fetch_history_kraken(Tier::Intermediate).unwrap();
        create_kraken_txs(
            wallet_manager,
            &mut kraken_txs,
            response.0,
            response.1,
            response.2,
            response.3,
            pairs.get(),
        )
        .map_err(|e| IoError::new(e.to_string()))?;
        let file = File::create(file_path).expect("Unable to create file");
        let mut writer = Serializer::new(file);
        kraken_txs
            .serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;
    } else {
        let file = File::open(file_path).map_err(|e| IoError::new(e.to_string()))?;
        let txs: Vec<Transaction> =
            rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
        kraken_txs = txs;
    }
    return Ok(kraken_txs);
}

pub fn kraken_pairs() -> Result<KrakenPairs, IoError> {
    let file_path = ".data/kraken/kraken_pairs";

    if !file_exists(file_path) {
        let asset_pairs_raw = fetch_assets_pair().unwrap().result.unwrap().pairs;
        let asset_pairs = map_asset_pairs(asset_pairs_raw);

        create_directories_if_needed(file_path);
        let file = File::create(file_path).expect("Unable to create file");
        let mut writer = Serializer::new(file);
        asset_pairs
            .serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;

        return Ok(asset_pairs);
    } else {
        let file = File::open(file_path).map_err(|e| IoError::new(e.to_string()))?;
        let deserialized_map: KrakenPairs =
            rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
        return Ok(deserialized_map);
    }
}

// pub async fn get_price_api(time: DateTime<Utc>, currency: String) -> Option<Decimal> {

// }
