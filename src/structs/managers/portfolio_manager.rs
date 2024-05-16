use hashbrown::HashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::structs::{Transaction, TransactionId, Wallet, WalletId};

/* This manager is used for associating the price and the balance of a wallet at a time "t" => associated with a transaction
This allow to save the history of price of the asset for calculating the full value of the portfolio without
having to call API again often.

If only one wallet is added, then we only have to call one API for this specific Wallet for each taxable transaction.

We cannot add this information on a tx as we need to keep track of all the wallets that have a non null balances for each taxable tx.
*/
#[derive(Serialize, Deserialize)]
pub struct PortfolioManager {
    pub portfolio_history: HashMap<TransactionId, HashMap<WalletId, WalletSnapshot>>, // We store only for taxable Transactions
}

#[derive(Serialize, Deserialize)]
pub struct WalletSnapshot {
    balance: Decimal,
    price_eur: Option<Decimal>,
}

impl PortfolioManager {
    // Updates

    // save
    // drop trait

    pub fn calculate_portfolio_history(txs: Vec<Transaction>) {
        for tx in txs {
            // Calculate the state of the transaction
            // determine if it needs to be stored and if price needs to be taken : if yes, get each wallet in list and call the API corresponding to the platform of the wallet
            //                                                                              Store the state in the PortfolioManager,
            // Keep the state for the next transaction
        }
    }

    fn _calculate(
        &self,
        tx: &Transaction,
        previous_tx: &Transaction,
        wallets: &HashMap<String, Wallet>,
    ) {
        // Should need some kind of copy of the wallets for not changing the actual data.
        match tx {
            Transaction::Trade {
                from,
                to,
                sold_amount,
                bought_amount,
                taxable,
                ..
            } => {
                let from = wallets.get(from);
                let to = wallets.get(to);
                if let Some(Wallet::Crypto(base)) = from {
                    let wallet_snap = self.portfolio_history.get(&previous_tx.get_tx_base().id);
                }
            }
            Transaction::Transfer {
                tx,
                from,
                to,
                amount,
                taxable,
                cost_basis,
            } => {
                // let [from_wallet, to_wallet] = wallets.get_many_mut([from, to]).unwrap();
                // from_wallet.get_mut().balance -= amount;
                // to_wallet.get_mut().balance += amount;
            }
            Transaction::Deposit { tx, to, amount } => {
                todo!()
            }
            Transaction::Withdrawal { tx, from, amount } => todo!(),
        }
    }
}
