use hashbrown::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    api::get_price_api,
    errors::PortfolioHistoryError,
    structs::{PortfolioWalletSnapshot, Transaction, TransactionId, Wallet, WalletId, WalletSnapshot},
};

use super::Persistable;

/* This manager is used for associating the price and the balance of a wallet at a time "t" => associated with a transaction
This allow to save the history of price of the asset for calculating the full value of the portfolio without
having to call API again often.

If only one wallet is added, then we only have to call one API for this specific Wallet for each taxable transaction.
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioManager {
    pub portfolio_history: HashMap<TransactionId, Portfolio>, // We store only for taxable Transactions
    path: String,
    persist: bool
}

/*The pf_total_value should be set depending on the global value of the portfolio before each transaction (at least each taxable one).
It can be caculated from the price of all wallets at an instant t.
The issue is getting price at instant t may take time (calling API). We want to get that information before actually treating the transaction,
when we only want it for taxable events.*/
#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio{
    pub tx_id : TransactionId,
    pub wallet_snaps : HashMap<WalletId, PortfolioWalletSnapshot>,
    pub is_taxable : bool,
    pub pf_total_value: Decimal,      // Portfolio total value in euro
    pub is_pf_total_calculated: bool, // Each time recalculation is needed, this should be set to false (Recalculation use the PortfolioManager)
}

impl Portfolio {
    pub fn new(tx_id: String,is_taxable: bool) -> Self{
        Self { tx_id, wallet_snaps: HashMap::new(), is_taxable, pf_total_value: dec!(0), is_pf_total_calculated: false }
    }
}


impl Persistable for PortfolioManager {
    const PATH:  &'static str = ".data/portfolio_history";

    fn default_new(path: String, persist: bool) -> Self {
        Self {
            portfolio_history: HashMap::new(),
            path,
            persist
        }
    }

    fn get_path(&self) -> &str{
        return &self.path;
    }

    fn is_persistent(&self) -> bool {
        return self.persist;
    }
}

impl Drop for PortfolioManager {
    fn drop(&mut self) {
        if self.persist{
            let _save = self.save();
        }
    }
}


impl PortfolioManager {

    #[tokio::main]
    pub async fn calculate_portfolio_history(
        &mut self,
        txs: &Vec<Transaction>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        let mut state: HashMap<WalletId, PortfolioWalletSnapshot> = HashMap::new();
        for tx in txs {
            let is_taxable = tx.is_taxable();
            let tx_id = tx.get_tx_base().id.clone();
            if  self.portfolio_history.get(&tx_id).is_none(){
                self.portfolio_history.insert(tx_id.to_string(), Portfolio::new(tx_id.to_string(),is_taxable));
            }
            self._calculate(tx, is_taxable,&mut state, wallets).await?;
            if is_taxable{
                let pf_total_value = self.calculate_total_value(&tx_id).unwrap();
                let portfolio = self.portfolio_history.get_mut(&tx_id).unwrap();
                portfolio.pf_total_value = pf_total_value;
                portfolio.is_pf_total_calculated = true;
            }
        }
        Ok(())
    }

    pub fn calculate_total_value(&self, tx_id: &TransactionId) -> Option<Decimal> {
        return self.portfolio_history.get(tx_id).map(|portfolio| {
            portfolio.wallet_snaps.values().fold(Decimal::new(0, 0), |acc, wallet| {
                acc + wallet.pre_tx_balance * wallet.price_eur.unwrap()
            })
        });
    }

    async fn _calculate(
        &mut self,
        transaction: &Transaction,
        is_taxable: bool,
        previous_state: &mut HashMap<WalletId, PortfolioWalletSnapshot>,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<(), PortfolioHistoryError> {
        match transaction {
            Transaction::Trade {
                tx,
                from,
                to,
                sold_amount,
                bought_amount,
                ..
            } => {
                let from_wallet = wallets.get(&from.id);
                let to_wallet = wallets.get(&to.id);

                self.insert_balance_from_wallet(&tx.id,from_wallet, from, previous_state)?;

                if is_taxable {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    let portfolio = self.portfolio_history.get_mut(&tx.id).unwrap();
                    portfolio.wallet_snaps = new_state;
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
                ..
            } => {
                let from_wallet = wallets.get(&from.id);
                let to_wallet = wallets.get(&to.id);

                self.insert_balance_from_wallet(&tx.id,from_wallet, from, previous_state)?;

                if is_taxable {
                    // If taxable we need the price and to insert/update the history
                    let new_state = self
                        .get_price_if_needed(previous_state, transaction, wallets)
                        .await?;
                    let portfolio = self.portfolio_history.get_mut(&tx.id).unwrap();
                    portfolio.wallet_snaps = new_state;
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
        previous_state: &mut HashMap<WalletId, PortfolioWalletSnapshot>,
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
                } else {
                    self.portfolio_history.get_mut(tx_id).unwrap().wallet_snaps.insert(base.id.clone(), from_snap.to_portfolio());
                }
            } else {
                let snap = from_snap.to_portfolio();
                previous_state.insert(base.id.clone(), snap.clone());
                self.portfolio_history.get_mut(tx_id).unwrap().wallet_snaps.insert(base.id.clone(), snap);
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
        previous_state: &mut HashMap<WalletId, PortfolioWalletSnapshot>,
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
        previous_state: &mut HashMap<WalletId, PortfolioWalletSnapshot>,
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
                    PortfolioWalletSnapshot {
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
        state: &mut HashMap<WalletId, PortfolioWalletSnapshot>,
        transaction: &Transaction,
        wallets: &HashMap<String, Wallet>,
    ) -> Result<HashMap<WalletId, PortfolioWalletSnapshot>, PortfolioHistoryError> {
        let tx = transaction.get_tx_base();
        let existing_state = self.portfolio_history.get(&tx.id);
        for (id, wallet_snap) in &mut *state {
            if let Some(ref portfolio) = existing_state {
                // If the state existed before, we can try to get the previous calculated prices
                let previous_wallet = portfolio.wallet_snaps.get(id);
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


