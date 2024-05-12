use std::collections::HashMap;

use crate::{
    api::{fetch_history_kraken, Tier},
    structs::{
        transaction::{self, Transaction},
        Wallet, WalletMap,
    },
};
pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
use api::create_kraken_txs;
use dotenv::dotenv;

// use crate::structs::Transaction;

fn main() {
    dotenv().ok();

    let mut wallets: WalletMap = HashMap::new();
    let transactions: Vec<Transaction> = Vec::new();

    let response = fetch_history_kraken(Tier::Intermediate).unwrap();
    println!("{:?}", response);
    create_kraken_txs(wallets, transactions, response.0, response.1, response.2);
}
