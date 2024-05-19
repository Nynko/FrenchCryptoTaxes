use std::fs::File;

use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use rmp_serde::Serializer;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        create_kraken_txs, fetch_assets_pair, fetch_history_kraken, map_asset_pairs, Deposit,
        HistoryResponse, KrakenPairs, LedgerHistory, Tier, TradeInfo, Withdrawal,
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

    let file_path = ".data/kraken/kraken_mapped_data";
    if !file_exists(file_path) {
        let response = get_kraken_history()?;
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

        println!("{:?}", kraken_txs);
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

pub fn get_kraken_history() -> Result<HistoryResponse, IoError> {
    let file_path = ".data/kraken/kraken_history";

    if !file_exists(file_path) {
        let response = fetch_history_kraken(Tier::Intermediate).unwrap();

        create_directories_if_needed(file_path);
        let file = File::create(file_path).expect("Unable to create file");
        let mut writer = Serializer::new(file);
        response
            .serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;

        return Ok(response);
    } else {
        let file = File::open(file_path).map_err(|e| IoError::new(e.to_string()))?;
        let deserialized_map: HistoryResponse =
            rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
        return Ok(deserialized_map);
    }
}

fn load_if_exist<T: for<'de> Deserialize<'de>>(file_path: &str) -> Result<Option<T>, IoError> {
    if file_exists(file_path) {
        let file = File::open(file_path).map_err(|e| IoError::new(e.to_string()))?;
        let deserialized_map: T =
            rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
        return Ok(Some(deserialized_map));
    }
    return Ok(None);
}
