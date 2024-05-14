use std::collections::HashMap;

use crate::{
    api::{fetch_history_kraken, kraken_pairs, map_asset_pairs, Tier},
    structs::{
        transaction::{self, Transaction},
        wallet, Wallet, WalletIdMap,
    },
};
pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
pub mod utils;
use api::{create_kraken_txs, fetch_assets_pair, fetch_specific_trade_data, handle_kraken_data};
use dotenv::dotenv;
use structs::{TransactionManager, WalletManager};

// use crate::structs::Transaction;

fn main() {
    dotenv().ok();

    let mut wallet_manager = WalletManager::new().unwrap();
    let mut transactions_manager = TransactionManager::new().unwrap();

    let kraken_txs = handle_kraken_data(&mut wallet_manager).unwrap();
    transactions_manager.extend(kraken_txs);
}
