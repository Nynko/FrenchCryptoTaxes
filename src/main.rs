pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
pub mod utils;
use api::handle_kraken_data;
use dotenv::dotenv;
use structs::{TransactionManager, WalletManager};

fn main() {
    dotenv().ok();

    let mut wallet_manager = WalletManager::new().unwrap();
    let mut transactions_manager = TransactionManager::new().unwrap();

    let kraken_txs = handle_kraken_data(&mut wallet_manager).unwrap();
    transactions_manager.extend_update(kraken_txs);

    let txs = transactions_manager.get();
    println!("{:?}",txs);
}
