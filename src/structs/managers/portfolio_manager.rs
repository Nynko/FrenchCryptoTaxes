use std::fs::{self, File};

use hashbrown::HashMap;
use rmp_serde::Serializer;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    api::get_price_api,
    errors::{IoError, PortfolioHistoryError},
    structs::{Transaction, TransactionId, Wallet, WalletId, WalletSnapshot},
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
    path: String,
}

impl PortfolioManager {
    pub const PATH: &'static str = ".data/portfolio_history";

    pub fn new(path: Option<String>) -> Result<Self, IoError> {
        // Load wallets here or create empty Vec
        let path = path.unwrap_or(Self::PATH.to_string());
        if !file_exists(&path) {
            return Ok(Self {
                portfolio_history: HashMap::new(),
                path,
            });
        } else {
            let file = File::open(path).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized_map: PortfolioManager =
                rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
            return Ok(deserialized_map);
        }
    }

    pub fn save(&self) -> Result<(), IoError> {
        create_directories_if_needed(&self.path);
        let file = File::create(&self.path).map_err(|e| IoError::new(e.to_string()))?;
        let mut writer = Serializer::new(file);
        self.serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;
        return Ok(());
    }

    pub fn delete(&self) -> Result<(), IoError> {
        if file_exists(&self.path) {
            fs::remove_file(&self.path).map_err(|e| IoError::new(e.to_string()))?;
        }
        Ok(())
    }

    #[tokio::main]
    pub async fn calculate_portfolio_history_and_update_tx(
        &mut self,
        txs: &mut Vec<Transaction>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        let mut state: HashMap<WalletId, WalletSnapshot> = HashMap::new();
        for tx in txs {
            if  self.portfolio_history.get(&tx.get_tx_base().id).is_none(){
                self.portfolio_history.insert(tx.get_tx_base().id.to_string(), HashMap::new());
            }
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
            wallet_map.values().fold(Decimal::new(0, 0), |acc, wallet| {
                acc + wallet.pre_tx_balance * wallet.price_eur.unwrap()
            })
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
            if  self.portfolio_history.get(&tx.get_tx_base().id).is_none(){
                self.portfolio_history.insert(tx.get_tx_base().id.to_string(), HashMap::new());
            }
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
                let from_wallet = wallets.get(&from.id);
                let to_wallet = wallets.get(&to.id);

                self.insert_balance_from_wallet(&tx.id,from_wallet, from, previous_state)?;

                if taxable.as_ref().is_some_and(|tax| tax.is_taxable) {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    self.portfolio_history.insert(tx.id.clone(), new_state);
                }

                self.update_balance_from_wallet(
                    from_wallet,
                    sold_amount,
                    tx.id.clone(),
                    previous_state,
                )?;
                self.update_balance_to_wallet(to_wallet, to, bought_amount, previous_state);
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
                let from_wallet = wallets.get(&from.id);
                let to_wallet = wallets.get(&to.id);

                self.insert_balance_from_wallet(&tx.id,from_wallet, from, previous_state)?;

                if taxable.as_ref().is_some_and(|tax| tax.is_taxable) {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    self.portfolio_history.insert(tx.id.clone(), new_state);
                }

                self.update_balance_from_wallet(
                    from_wallet,
                    amount,
                    tx.id.clone(),
                    previous_state,
                )?;
                self.update_balance_to_wallet(to_wallet, to, amount, previous_state);

                Ok(())
            }
            _ => Ok(()), // We don't need information when it is fiat
        }
    }

    fn insert_balance_from_wallet(
        &mut self,
        tx_id: &String,
        from: Option<&Wallet>,
        from_snap: &WalletSnapshot,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
    ) -> Result<(), PortfolioHistoryError> {
        if let Some(Wallet::Crypto(base)) = from {
            let previous_snap = previous_state.get_mut(&base.id);
            if let Some(prev_snap) = previous_snap {
                if (prev_snap.pre_tx_balance != from_snap.pre_tx_balance) {
                    // return Err(PortfolioHistoryError::MismatchBetweenBalances {
                    //     threshold: dec!(0),
                    //     old_balance: prev_snap.balance,
                    //     new_balance: from_snap.balance,
                    // });
                    println!(
                        "Mismatch calculated balance: {} - balance from data {} - wallet_id: {} - for currency: {}",
                        prev_snap.pre_tx_balance, from_snap.pre_tx_balance,base.id,base.currency
                    );
                } else if from_snap.price_eur.is_some(){
                    self.portfolio_history.get_mut(tx_id).unwrap().insert(base.id.clone(), from_snap.clone());
                }
            } else {
                previous_state.insert(base.id.clone(), from_snap.clone());
                self.portfolio_history.get_mut(tx_id).unwrap().insert(base.id.clone(), from_snap.clone());
            }
        }
        Ok(())
    }

    /* As a rule, the fee is not contained in the amount in any way:

    Example:
            Sold_Amount: 100
            From Wallet :
                pre_tx_balance: 150
                fee: 2
                post_tx_balance: 48 (pre_tx_balance - amount - fee)

            Bought_Amount: 220
            To Wallet:
                pre_tx_balance: 10
                fee: 1
                post_tx_balance: 229 (pre_tx_balance + amount - fee)
     */

    fn update_balance_from_wallet(
        &self,
        from: Option<&Wallet>,
        amount: &Decimal,
        tx_id: TransactionId,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
    ) -> Result<(), PortfolioHistoryError> {
        if let Some(Wallet::Crypto(base)) = from {
            let wallet_snap = previous_state.get_mut(&base.id);
            let snap = wallet_snap.unwrap(); // We added it before so it must exist
            let fee = snap.fee.unwrap_or(dec!(0));
            snap.pre_tx_balance = snap.pre_tx_balance - amount - fee;
            if snap.pre_tx_balance == dec!(0) {
                // No need to keep the walletSnapshot if the balance is zero
                previous_state.remove(&base.id);
            }
        }
        Ok(())
    }

    fn update_balance_to_wallet(
        &self,
        to: Option<&Wallet>,
        tx_wallet_snap: &WalletSnapshot,
        amount: &Decimal,
        previous_state: &mut HashMap<WalletId, WalletSnapshot>,
    ) {
        if let Some(Wallet::Crypto(base)) = to {
            let wallet_snap = previous_state.get_mut(&base.id);
            if let Some(snap) = wallet_snap {
                let fee = snap.fee.unwrap_or(dec!(0));
                snap.pre_tx_balance += amount - fee;
            } else {
                let fee = tx_wallet_snap.fee.unwrap_or(dec!(0));
                previous_state.insert(
                    base.id.clone(),
                    WalletSnapshot {
                        id: base.id.to_string(),
                        pre_tx_balance: tx_wallet_snap.pre_tx_balance + *amount - fee,
                        fee: tx_wallet_snap.fee,
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
        let existing_state = self.portfolio_history.get(&tx.id);
        for (id, wallet_snap) in &mut *state {
            if let Some(ref prev) = existing_state {
                // If the state existed before, we can try to get the previous calculated prices
                let previous_wallet = prev.get(id);
                if previous_wallet.is_some() && previous_wallet.unwrap().price_eur.is_some() {
                    // Update the state with existing values price
                    wallet_snap.price_eur = previous_wallet.unwrap().price_eur;
                    continue; // No need to get the price
                }
            }

            // Else: if the price didn't exist before OR the wallet didn't exist: get the price
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
