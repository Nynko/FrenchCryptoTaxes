pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
pub mod utils;
use api::handle_kraken_data;
use dotenv::dotenv;
use functions::calculate_full_cost_basis;
use structs::{TransactionManager, WalletManager};

use crate::structs::portfolio_manager::{self, PortfolioManager};

fn main() {
    dotenv().ok();

    let mut wallet_manager = WalletManager::new().unwrap();
    let mut transactions_manager = TransactionManager::new().unwrap();
    let mut portfolio_manager = PortfolioManager::new().unwrap();

    let kraken_txs = handle_kraken_data(&mut wallet_manager).unwrap();
    transactions_manager.extend_update(kraken_txs);

    transactions_manager.sort();

    portfolio_manager
        .calculate_portfolio_history_and_update_tx(
            transactions_manager.get_mut(),
            &wallet_manager.wallets,
        )
        .unwrap();

    println!("{:?}", portfolio_manager.portfolio_history);

    // calculate_full_cost_basis(transactions_manager.get_mut());
}
