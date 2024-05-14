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

// use crate::structs::Transaction;

fn main() {
    dotenv().ok();

    let mut wallet_ids: WalletIdMap = WalletIdMap::new();
    let mut wallets: HashMap<String, Wallet> = HashMap::new();
    let mut transactions: Vec<Transaction> = Vec::new();

    handle_kraken_data(&mut wallet_ids,& mut wallets,&mut transactions).unwrap();


}
