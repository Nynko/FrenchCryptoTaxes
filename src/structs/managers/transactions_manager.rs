use hashbrown::HashSet;
use std::fs::{self, File};

use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

use crate::{
    errors::IoError,
    structs::{Transaction, TransactionId},
    utils::{create_directories_if_needed, file_exists},
};

/* This transaction manager will handle saving the data and loading the previous data if they exist, the merging
of data and it will implement de Drop trait to save when reference is dropped */
#[derive(Serialize, Deserialize)]
pub struct TransactionManager {
    transactions: Vec<Transaction>, // Suggestion: Implement a SortedVec maybe
    hash_set: HashSet<TransactionId>, // HashSet for merging Vec<String> and preventing duplicates, the ID must comes from external sources OR deterministically created from "uniqueness" element of the transaction
    path: String,
}

impl TransactionManager {
    pub const PATH: &'static str = ".data/transactions";

    pub fn new(path: Option<String>) -> Result<Self, IoError> {
        // Load wallets here or create empty Vec
        let path = if path.is_some() {
            path.unwrap()
        } else {
            Self::PATH.to_string()
        };
        if !file_exists(&path) {
            return Ok(Self {
                transactions: Vec::new(),
                hash_set: HashSet::new(),
                path,
            });
        } else {
            let file = File::open(path).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized_map: TransactionManager =
                rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
            return Ok(deserialized_map);
        }
    }

    pub fn get(&self) -> &Vec<Transaction> {
        return &self.transactions;
    }

    pub fn get_mut(&mut self) -> &mut Vec<Transaction> {
        return &mut self.transactions;
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

    /* Add transaction by avoiding duplicates */
    pub fn push(&mut self, tx: Transaction) {
        if self.hash_set.insert(tx.get_tx_base().id.clone()) {
            self.transactions.push(tx);
        }
    }

    /* Extends transaction by avoiding duplicates */
    pub fn extend(&mut self, txs: Vec<Transaction>) {
        for tx in txs {
            self.push(tx); // We could optimize using extend_slice until the first duplicate (hence most of the time it would extend by slice everything)
        }
    }

    /* Add transaction by updating transaction that are equals by Hash implementation */
    pub fn push_update(&mut self, tx: Transaction) {
        if self.hash_set.insert(tx.get_tx_base().id.clone()) {
            self.transactions.push(tx);
        } else {
            // if the value is already there -> update
            let index = self
                .transactions
                .binary_search_by_key(&tx.get_tx_base().timestamp, |trans| {
                    trans.get_tx_base().timestamp
                })
                .unwrap();
            self.transactions[index] = tx;
        }
    }

    /* Extends transaction by updating duplicates */
    pub fn extend_update(&mut self, txs: Vec<Transaction>) {
        for tx in txs {
            self.push_update(tx); // We could optimize using extend_slice until the first duplicate (hence most of the time it would extend by slice everything)
        }
    }

    pub fn sort(&mut self) {
        self.transactions
            .sort_by(|a, b| a.get_tx_base().timestamp.cmp(&b.get_tx_base().timestamp))
    }
}

impl Drop for TransactionManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}

#[cfg(test)]
mod tests {

    use std::{thread::sleep, time::Duration};

    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;
    use serial_test::serial;

    use crate::structs::{
        wallet::Platform, GlobalCostBasis, Taxable, TradeType, TransactionBase, WalletSnapshot,
    };

    use super::*;

    #[test]
    #[serial]
    fn test_unicity() {
        let mut tx_manager =
            TransactionManager::new(Some(".data_test/transactions".to_string())).unwrap();

        let tx1 = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: DateTime::from_timestamp(61, 0).unwrap(),
            },
            from: WalletSnapshot {
                id: "btc".to_string(),
                balance: dec!(1),
                price_eur: None,
            },
            to: WalletSnapshot {
                id: "eur".to_string(),
                balance: dec!(0),
                price_eur: None,
            },
            exchange_pair: None,
            sold_amount: dec!(1),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
            taxable: Some(Taxable {
                is_taxable: true,
                price_eur: dec!(1),
                pf_total_value: dec!(1300),
                is_pf_total_calculated: true,
            }),
            cost_basis: GlobalCostBasis {
                pf_cost_basis: dec!(0),
                pf_total_cost: dec!(0),
            },
        };

        let tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: DateTime::from_timestamp(61, 0).unwrap(),
            },
            from: WalletSnapshot {
                id: "btc".to_string(),
                balance: dec!(1),
                price_eur: None,
            },
            to: WalletSnapshot {
                id: "eur".to_string(),
                balance: dec!(0),
                price_eur: None,
            },
            exchange_pair: None,
            sold_amount: dec!(1),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
            taxable: None,
            cost_basis: GlobalCostBasis {
                pf_cost_basis: dec!(0),
                pf_total_cost: dec!(0),
            },
        };

        assert_ne!(tx1, tx2);

        tx_manager.push(tx1.clone());
        tx_manager.push(tx2.clone());

        assert_eq!(tx_manager.transactions.len(), 1);
        assert_eq!(tx_manager.transactions[0], tx1);

        tx_manager.delete();
    }

    #[test]
    #[serial]
    fn test_update() {
        let mut tx_manager =
            TransactionManager::new(Some(".data_test/transactions2".to_string())).unwrap();

        let tx1 = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: DateTime::from_timestamp(61, 0).unwrap(),
            },
            from: WalletSnapshot {
                id: "btc".to_string(),
                balance: dec!(1),
                price_eur: None,
            },
            to: WalletSnapshot {
                id: "eur".to_string(),
                balance: dec!(0),
                price_eur: None,
            },
            exchange_pair: None,
            sold_amount: dec!(1),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
            taxable: Some(Taxable {
                is_taxable: true,
                price_eur: dec!(1),
                pf_total_value: dec!(1300),
                is_pf_total_calculated: true,
            }),
            cost_basis: GlobalCostBasis {
                pf_cost_basis: dec!(0),
                pf_total_cost: dec!(0),
            },
        };

        let tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: DateTime::from_timestamp(61, 0).unwrap(),
            },
            from: WalletSnapshot {
                id: "btc".to_string(),
                balance: dec!(1),
                price_eur: None,
            },
            to: WalletSnapshot {
                id: "eur".to_string(),
                balance: dec!(0),
                price_eur: None,
            },
            exchange_pair: None,
            sold_amount: dec!(1),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
            taxable: None,
            cost_basis: GlobalCostBasis {
                pf_cost_basis: dec!(0),
                pf_total_cost: dec!(0),
            },
        };

        assert_ne!(tx1, tx2);

        tx_manager.push_update(tx1.clone());
        tx_manager.push_update(tx2.clone());

        assert_eq!(tx_manager.transactions.len(), 1);
        assert_eq!(tx_manager.transactions[0], tx2);
    }
}
