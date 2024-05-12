use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::{collections::HashMap, str::FromStr};

use crate::{
    api::{Deposit, EntryType, LedgerEntry, Withdrawal},
    errors::MappingError,
    structs::{Currency, Transaction, TransactionBase, Wallet, WalletMap},
};

/* This function take existing currencies, wallets and Transactions and add the new elements  */
pub fn create_kraken_txs(
    currencies: HashMap<&str, Currency>,
    wallets: WalletMap,
    txs: Vec<Transaction>,
    ledger: Vec<LedgerEntry>,
    deposits: HashMap<String, Deposit>,
    withdrawals: HashMap<String, Withdrawal>,
) -> Result<(), MappingError> {
    let mut index = 0;
    while index < ledger.len() {
        let entry = &ledger[index];
        match entry.r#type {
            EntryType::Trade => {
                let refid = &entry.refid;
                let matching_entry = &ledger[index + 1];
                if matching_entry.refid != *refid {
                    // Yeah, too lazy to do a proper check along the whole vec, this case shouldn't happen
                    println!("{:?}", ledger[index - 1]);
                    return Err(MappingError::new(String::from("We have unmatching refid, meaning either something appeared between a trade or data are broken.
                    Either way, show this error to the dev on github with your data, and we will help you. This shouldn't happen")));
                }
                index += 1;
            }
            // EntryType::Transfer => todo!(),
            // EntryType::Deposit => todo!(),
            // EntryType::Withdrawal => todo!(),
            // EntryType::Staking => todo!(),
            // EntryType::Reward => todo!(),
            _ => (),
        }
        index += 1;
    }
    Ok(())
}

// pub fn get_transactions<'a>(ledger: &'a Vec<LedgerEntry>, wallet_from: &'a Wallet, wallet_to: &'a Wallet) -> (Vec<Transaction<'a,'a>>, Vec<>){
//     let mut transactions = Vec::new();
//     for entry in ledger{
//         // Convert the f64 timestamp to seconds and nanoseconds
//         let seconds = ledger.time.trunc() as i64;
//         let nanoseconds = ((ledger.time.fract() * 1_000_000_000.0) as u32).max(0);
//         let tx = Transaction::Trade { tx: TransactionBase{
//             id: ledger.refid,
//             fee:Some(Decimal::from_str(&ledger.fee).unwrap()),
//             timestamp: DateTime::<Utc>::from_timestamp(seconds,nanoseconds).expect("Error when parsin time"),
//             is_taxable: false, // false first
//             fee_price: todo!(),
//         }, from: wallet_from, to: wallet_to, sold_amount: trade., bought_amount: (), bought_price_eur: (), pf: () };
//         transactions.push(tx);
//     }
//     return transactions;

// }
