pub mod api;
pub mod errors;
pub mod functions;
pub mod parsing;
pub mod structs;
pub mod tests;
pub mod utils;
use api::handle_kraken_data;
use dotenv::dotenv;
use functions::{calculate_full_cost_basis, calculate_tax_gains};
use structs::{Transaction, TransactionManager, WalletManager};

use crate::structs::PortfolioManager;

fn main() {
    dotenv().ok();

    let mut wallet_manager = WalletManager::new(None).unwrap();
    let mut transactions_manager = TransactionManager::new(None).unwrap();
    let mut portfolio_manager = PortfolioManager::new(None).unwrap();

    let kraken_txs = handle_kraken_data(&mut wallet_manager).unwrap();
    transactions_manager.extend_update(kraken_txs);

    transactions_manager.sort();

    // println!("{:?}", transactions_manager.get());

    portfolio_manager
        .calculate_portfolio_history_and_update_tx(
            transactions_manager.get_mut(),
            &wallet_manager.wallets,
        )
        .unwrap();

    // println!("{:?}", portfolio_manager.portfolio_history);

    calculate_full_cost_basis(transactions_manager.get_mut());

    for tx in transactions_manager.get() {
        match tx {
            Transaction::Trade { taxable, .. } | Transaction::Transfer { taxable, .. } => {
                if taxable.as_ref().is_some_and(|tax| tax.is_taxable) {
                    let tax = calculate_tax_gains(tx);
                    println!("tax: {tax}");
                } else {
                    ()
                }
            }
            _ => (),
        }
    }
}
