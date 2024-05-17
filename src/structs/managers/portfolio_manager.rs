use std::fs::File;

use hashbrown::HashMap;
use rmp_serde::Serializer;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    api::get_price_api,
    errors::{IoError, PortfolioHistoryError},
    structs::{Transaction, TransactionId, Wallet, WalletId},
    utils::{create_directories_if_needed, file_exists},
};

/* This manager is used for associating the price and the balance of a wallet at a time "t" => associated with a transaction
This allow to save the history of price of the asset for calculating the full value of the portfolio without
having to call API again often.

If only one wallet is added, then we only have to call one API for this specific Wallet for each taxable transaction.
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioManager {
    pub portfolio_history: HashMap<TransactionId, HashMap<WalletId, WalletSnapshot>>, // We store only for taxable Transactions
}

/* Correspond to a snapshot of the wallet state (balance and potentially price) for a transaction */
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub balance: Decimal,
    pub price_eur: Option<Decimal>,
}

impl PortfolioManager {
    pub const PATH: &'static str = ".data/portfolio_history";

    pub fn new() -> Result<Self, IoError> {
        // Load wallets here or create empty Vec
        if !file_exists(Self::PATH) {
            return Ok(Self {
                portfolio_history: HashMap::new(),
            });
        } else {
            let file = File::open(Self::PATH).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized_map: PortfolioManager =
                rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
            return Ok(deserialized_map);
        }
    }

    pub fn save(&self) -> Result<(), IoError> {
        create_directories_if_needed(Self::PATH);
        let file = File::create(Self::PATH).map_err(|e| IoError::new(e.to_string()))?;
        let mut writer = Serializer::new(file);
        self.serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;
        return Ok(());
    }

    #[tokio::main]
    pub async fn calculate_portfolio_history_and_update_tx(
        &mut self,
        txs: &mut Vec<Transaction>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        let mut state: HashMap<WalletId, WalletSnapshot> = HashMap::new();
        for tx in txs {
            self._calculate(tx, &mut state, wallets).await?;
            match tx {
                Transaction::Trade {
                    tx: base, taxable, ..
                }
                | Transaction::Transfer {
                    tx: base, taxable, ..
                } => {
                    if let Some(tax) = taxable {
                        if tax.is_taxable {
                            tax.pf_total_value = self.calculate_total_value(&base.id).unwrap();
                            tax.is_pf_total_calculated = true;
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    pub fn calculate_total_value(&self, tx_id: &TransactionId) -> Option<Decimal> {
        return self.portfolio_history.get(tx_id).map(|wallet_map| {
            wallet_map
                .values()
                .fold(Decimal::new(0, 0), |acc, wallet| acc + wallet.balance)
        });
    }

    #[tokio::main]
    pub async fn calculate_portfolio_history(
        &mut self,
        txs: &Vec<Transaction>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        let mut state: HashMap<WalletId, WalletSnapshot> = HashMap::new();
        for tx in txs {
            self._calculate(tx, &mut state, wallets).await?;
        }
        Ok(())
    }

    /* Return is_taxable */
    async fn _calculate(
        &mut self,
        transaction: &Transaction,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        match transaction {
            Transaction::Trade {
                tx,
                from,
                to,
                taxable,
                sold_amount,
                bought_amount,
                ..
            } => {
                let from = wallets.get(from);
                let to = wallets.get(to);

                if taxable.as_ref().is_some_and(|tax| tax.is_taxable) {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    self.portfolio_history.insert(tx.id.clone(), new_state);
                }

                self.update_balance_from_wallet(from, sold_amount, tx.id.clone(), previous_state)?;
                self.update_balance_to_wallet(to, bought_amount, previous_state);
                Ok(())
            }
            Transaction::Transfer {
                tx,
                from,
                to,
                amount,
                taxable,
                ..
            } => {
                if taxable.as_ref().is_some_and(|tax| tax.is_taxable) {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    self.portfolio_history.insert(tx.id.clone(), new_state);
                }

                let from = wallets.get(from);
                let to = wallets.get(to);
                self.update_balance_from_wallet(from, amount, tx.id.clone(), previous_state)?;
                self.update_balance_to_wallet(to, amount, previous_state);

                Ok(())
            }
            _ => Ok(()), // We don't need information when it is fiat
        }
    }

    fn update_balance_from_wallet(
        &self,
        from: Option<&Wallet>,
        amount: &Decimal,
        tx_id: TransactionId,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
    ) -> Result<(), PortfolioHistoryError> {
        if let Some(Wallet::Crypto(base)) = from {
            let wallet_snap = previous_state.get_mut(&base.id);
            if let Some(snap) = wallet_snap {
                snap.balance -= amount;

                if snap.balance == dec!(0) {
                    // No need to keep the walletSnapshot if the balance is zero
                    previous_state.remove(&base.id);
                }
            } else {
                // We considere the wallet is either not ours or since we don't have information, we considere the wallet to be empty now.
                println!("WARNING: Wallet {} is missing in tx: {tx_id}", base.id);
            }
        }
        Ok(())
    }

    fn update_balance_to_wallet(
        &self,
        to: Option<&Wallet>,
        amount: &Decimal,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
    ) {
        if let Some(Wallet::Crypto(base)) = to {
            let wallet_snap = previous_state.get_mut(&base.id);
            if let Some(snap) = wallet_snap {
                snap.balance += amount;
            } else {
                previous_state.insert(
                    base.id.clone(),
                    WalletSnapshot {
                        balance: *amount,
                        price_eur: None,
                    },
                );
            }
        }
    }

    async fn get_price_if_needed(
        &self,
        state: &mut HashMap<WalletId, WalletSnapshot>,
        transaction: &Transaction,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<HashMap<WalletId, WalletSnapshot>, PortfolioHistoryError> {
        let tx = transaction.get_tx_base();
        let previous_value = self.portfolio_history.get(&tx.id);
        // Either updating the state or insert it
        for (id, wallet_snap) in &mut *state {
            if let Some(ref prev) = previous_value {
                let previous_wallet = prev.get(id);
                if previous_wallet.is_some() && previous_wallet.unwrap().price_eur.is_some() {
                    // Update the state with previous values price
                    wallet_snap.price_eur = previous_wallet.unwrap().price_eur;
                    continue; // No need to get the price
                }
            }
            // Get the price
            let wallet = wallets.get(id).unwrap();
            let price = get_price_api(transaction, wallet)
                .await
                .map_err(|e| PortfolioHistoryError::FailureGettingPrice(e))?;
            wallet_snap.price_eur = Some(price);
        }

        return Ok(state.clone());
    }
}

impl Drop for PortfolioManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}
