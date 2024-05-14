use std::{collections::HashSet, fs::File};

use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

use crate::{
    errors::IoError,
    utils::{create_directories_if_needed, file_exists},
};

use super::transaction::Transaction;

/* This transaction manager will handle saving the data and loading the previous data if they exist, the merging
of data and it will implement de Drop trait to save */
#[derive(Serialize, Deserialize)]
pub struct TransactionManager {
    transactions: Vec<Transaction>,
    hash_set: HashSet<Transaction>, // HashSet for merging Vec<Transaction> and preventing duplicates
}

impl TransactionManager {
    pub const PATH: &'static str = ".data/transactions";

    pub fn new() -> Result<Self, IoError> {
        // Load wallets here or create empty Vec
        if !file_exists(Self::PATH) {
            return Ok(Self {
                transactions: Vec::new(),
                hash_set: HashSet::new(),
            });
        } else {
            let file = File::open(Self::PATH).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized_map: TransactionManager =
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

    /* Add transaction by avoiding duplicates */
    pub fn push(&mut self, tx: Transaction) {
        if self.hash_set.insert(tx.clone()) {
            self.transactions.push(tx);
        }
    }

    /* Extends transaction by avoiding duplicates */
    pub fn extend(&mut self, txs: Vec<Transaction>) {
        for tx in txs {
            self.push(tx); // We could optimize using extend_slice until the first duplicate (hence most of the time it would extend by slice everythin)
        }
    }
}

impl Drop for TransactionManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}
