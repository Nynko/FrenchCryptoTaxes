pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
pub mod tests;
pub mod utils;
use api::handle_kraken_data;
use dotenv::dotenv;
use functions::calculate_tax_gains;
use structs::{global_cost_basis_manager::GlobalCostBasisManager, Persistable, TransactionManager, WalletManager};

use crate::structs::PortfolioManager;

fn main() {
    dotenv().ok();

    let mut wallet_manager = WalletManager::new(None).unwrap();
    let mut transactions_manager = TransactionManager::new(None).unwrap();
    let mut portfolio_manager = PortfolioManager::new(None).unwrap();
    let mut global_cost_basis_manager = GlobalCostBasisManager::new(None).unwrap();

    let kraken_txs = handle_kraken_data(&mut wallet_manager).unwrap();
    transactions_manager.extend_update(kraken_txs);

    transactions_manager.sort();

    portfolio_manager
        .calculate_portfolio_history(
            transactions_manager.get(),
            &wallet_manager.wallets,
        )
        .unwrap();

    global_cost_basis_manager.calculate_full_cost_basis(transactions_manager.get(),&portfolio_manager.portfolio_history);

    for tx in transactions_manager.get() {
        if tx.is_taxable(){
            let tx_id = tx.get_id();
            let portfolio = portfolio_manager.portfolio_history.get(tx_id).unwrap();
            let cost_basis = global_cost_basis_manager.global_cost_basis_history.get(tx_id).unwrap();
            let tax = calculate_tax_gains(&tx, portfolio,cost_basis);
            println!("tax: {tax}");
        }
    }
}
