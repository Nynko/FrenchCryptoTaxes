use hashbrown::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{functions::calculate_weigted_price, structs::{GlobalCostBasis, TradeType, Transaction, TransactionId, WalletSnapshot}};

use super::{Persistable, Portfolio};


/* This manager is used for associating the global cost basis at a time "t" => associated with a transaction
This allow to save the history of global cost basis with drop implementation
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCostBasisManager {
    pub global_cost_basis_history : HashMap<TransactionId,GlobalCostBasis>,
    path: String,
    persist: bool,
}


impl GlobalCostBasisManager{

    pub fn calculate_full_cost_basis(&mut self, txs: &Vec<Transaction>, portfolios: &HashMap<TransactionId,Portfolio>) {
        let mut global_cost_basis = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };
        for tx in txs {
            self.global_cost_basis_history.insert(tx.get_id().to_string(), global_cost_basis.clone());
            let portfolio = portfolios.get(tx.get_id());
            global_cost_basis = self.calculate_cost_basis(tx, portfolio, global_cost_basis.clone());
        }
    }


    /* Calculate the cost_basis, here "acquisition_pf_net" */
    fn calculate_cost_basis(&self, tx: &Transaction, portfolio: Option<&Portfolio>, current_pf: GlobalCostBasis) -> GlobalCostBasis {
        match tx {
            Transaction::Transfer {
                to,
                from,
                amount,
                ..
            } => {
                return self.calculate_new_cost_basis(to, from, portfolio, &current_pf, *amount);
            }
            Transaction::Trade {
                to,
                from,
                sold_amount,
                trade_type,
                ..
            } => {
                let added_cost = match trade_type {
                    TradeType::FiatToCrypto { local_cost_basis } => *local_cost_basis,
                    _ => dec!(0),
                };
                let new_gcs = self.calculate_new_cost_basis(to, from, portfolio, &current_pf, *sold_amount);
                return GlobalCostBasis{
                    pf_cost_basis: new_gcs.pf_cost_basis + added_cost,
                    pf_total_cost: new_gcs.pf_total_cost + added_cost,
                }
            }
            _ => current_pf, // ignoring the fiat deposit and withdrawal as they don't change the cost basis, they are here for accounting
        }
    }


    /* Calculate new Portfolio: if the transaction is taxable the new cost basis will change, otherwise only the fee might change it */
    fn calculate_new_cost_basis(
        &self,
        to: &WalletSnapshot,
        from: &WalletSnapshot,
        portfolio: Option<&Portfolio>,
        current_pf: &GlobalCostBasis,
        amount: Decimal,
    ) -> GlobalCostBasis {
        let current_cost_basis = current_pf.pf_cost_basis;
        let current_total_cost = current_pf.pf_total_cost;

        let fee = to.fee.unwrap_or(dec!(0)) * to.price_eur + from.fee.unwrap_or(dec!(0)) * from.price_eur;
        let mut cost_basis_adjustment: Decimal = dec!(0.00);
        if let Some(portfolio) = portfolio {
            if portfolio.is_taxable {
                // Selling of Crypto - Taxable event
                let sell_price: Decimal = Decimal::from(amount) * from.price_eur;
                let weigted_price =
                    calculate_weigted_price(sell_price, current_cost_basis, portfolio.pf_total_value);
                    
                cost_basis_adjustment = weigted_price;
            }
        }
        
        return GlobalCostBasis {
            pf_cost_basis: current_cost_basis - cost_basis_adjustment + fee,
            pf_total_cost: current_total_cost + fee,
        };
    }

}


impl Persistable for GlobalCostBasisManager {
    const PATH:  &'static str = ".data/global_cost_basis_history";

    fn default_new(path: String, persist:bool) -> Self {
        Self {
            global_cost_basis_history: HashMap::new(),
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

impl Drop for GlobalCostBasisManager {
    fn drop(&mut self) {
        if self.persist{
            let _save = self.save();
        }
    }
}



#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::functions::calculate_tax_gains;
    use crate::structs::{Owner, Platform, TransactionBase, Wallet, WalletBase, WalletSnapshot};

    use super::*;

    fn get_pf(cost_basis: Decimal, total_cost: Decimal) -> GlobalCostBasis {
        return GlobalCostBasis {
            pf_cost_basis: cost_basis,
            pf_total_cost: total_cost,
        };
    }

    fn create_wallets() -> (Wallet, Wallet, Wallet) {
        let btc = Wallet::Crypto(WalletBase {
            id: String::from("btc"),
            currency: "bitcoin".to_string(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: dec!(0),
            info: None,
        });

        let eur = Wallet::Fiat(WalletBase {
            id: String::from("eur"),
            currency: "euro".to_string(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: dec!(0),
            info: None,
        });

        let eth = Wallet::Crypto(WalletBase {
            id: String::from("eth"),
            currency: "ethereum".to_string(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: dec!(0),
            info: None,
        });

        (btc, eur, eth)
    }

    #[test]
    fn simple_transfer_with_fee() {
        let current_pf = get_pf(dec!(500.00), dec!(500.00));
        let (btc_wallet, _eur_wallet, _eth_wallet) = create_wallets();

        let price_eur_btc = dec!(64000.02);
        let fee = dec!(0.001);
        let fee_eur = fee * price_eur_btc;

        let mut tx = Transaction::Transfer {
            tx: TransactionBase {
                id: "test".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1),
                fee: Some(fee),
                price_eur: price_eur_btc,
            },
            to: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1),
                fee: None,
                price_eur: price_eur_btc,
            },
            amount: dec!(1),
            income: None,
        };

        let cost_basis_manager = GlobalCostBasisManager::new_non_persistent().unwrap();

        let new_pf = cost_basis_manager.calculate_cost_basis(&mut tx, None, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(500) + fee_eur);
        assert_eq!(new_pf.pf_cost_basis, dec!(500) + fee_eur);

    }

    #[test]
    fn simple_trades() {
        let current_pf = get_pf(dec!(18000), dec!(18000));
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(8),
                fee: None,
                price_eur: dec!(4000),
            },
            to: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1),
                fee: None,
                price_eur: dec!(1),
            },
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(5),
            bought_amount: dec!(20000),
            trade_type: TradeType::CryptoToFiat,
        };

        let portfolio = Portfolio {
            tx_id: "test".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(32000),
            is_pf_total_calculated: true,
        };

        let gains = calculate_tax_gains(&tx, &portfolio, &current_pf);
        assert_eq!(gains, dec!(8750));

        let cost_basis_manager = GlobalCostBasisManager::new_non_persistent().unwrap();
        let next_cost_basis = cost_basis_manager.calculate_cost_basis(&mut tx,Some(&portfolio), current_pf);

        assert_eq!(next_cost_basis.pf_total_cost, dec!(18000));
        assert_eq!(next_cost_basis.pf_cost_basis, dec!(18000) - dec!(11250));


    }

    #[test]
    fn simple_two_trades() {
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let mut tx0 = Transaction::Trade {
            tx: TransactionBase {
                id: "test0".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1000),
                fee: None,
                price_eur: dec!(1),
            },
            to: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(0),
                fee: None,
                price_eur: dec!(0), // No need for that 
            },
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1000),
            bought_amount: dec!(3),
            trade_type: TradeType::FiatToCrypto {
                local_cost_basis: dec!(1000),
            },
        };

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };
        let cost_basis_manager = GlobalCostBasisManager::new_non_persistent().unwrap();
        let current_pf = cost_basis_manager.calculate_cost_basis(&mut tx0, None,init_pf.clone());

        let tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(3),
                fee: None,
                price_eur: dec!(400),
            },
            to: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(0),
                fee: None,
                price_eur: dec!(1),
            },
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1.125),
            bought_amount: dec!(450),
            trade_type: TradeType::CryptoToFiat,
        };

        let portfolio = Portfolio {
            tx_id: "test".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(1200),
            is_pf_total_calculated: true,
        };

        let gains = calculate_tax_gains(&tx, &portfolio, &current_pf);
  
        assert_eq!(gains, dec!(75));

        let new_pf = cost_basis_manager.calculate_cost_basis(&tx, Some(&portfolio), current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(1000));
        assert_eq!(new_pf.pf_cost_basis, dec!(1000) - dec!(375));

        let mut tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: "test2".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1.875),
                fee: None,
                price_eur: dec!(693.33333333333333333333333333),
            },
            to: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(450),
                fee: None,
                price_eur: dec!(1),
            },
            exchange_pair: None,
            sold_amount: dec!(1.875),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
        };

        let portfolio2 = Portfolio {
            tx_id: "test2".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(1300),
            is_pf_total_calculated: true,
        };

        let gains = calculate_tax_gains(&tx2,&portfolio2, &new_pf);
        assert_eq!(gains, dec!(675));

        let new_pf2 = cost_basis_manager.calculate_cost_basis(&mut tx2, Some(&portfolio2), new_pf);

        assert_eq!(new_pf2.pf_total_cost, dec!(1000));

        assert_eq!(new_pf2.pf_cost_basis, dec!(0));


    }
}
