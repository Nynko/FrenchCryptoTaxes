use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    api::{fetch_specific_trade_data, AssetPair, Deposit, EntryType, LedgerEntry, Withdrawal},
    errors::MappingError,
    structs::{wallet::{Owner, Platform, WalletBase}, CurrentPortfolio, Transaction, TransactionBase, Wallet, WalletIdMap}, utils::generate_id,
};


/* This function take existing currencies, wallets and Transactions and add the new elements  */
#[tokio::main] // Async for calling API for getting asset price
pub async fn create_kraken_txs(
    wallet_ids: &mut WalletIdMap,
    wallets: &mut HashMap<String, Wallet>,
    txs: &mut Vec<Transaction>,
    ledger: Vec<LedgerEntry>,
    deposits: HashMap<String, Deposit>,
    withdrawals: HashMap<String, Withdrawal>,
    pairs: HashMap<(String, String), String>,
) -> Result<(), MappingError> {
    let platform = Platform::Kraken;
    let mut index = 0;
    while index < ledger.len() {
        let entry = &ledger[index];
        match entry.r#type {
            EntryType::Trade => {
                let refid = &entry.refid;
                let matching_entry = &ledger[index + 1];
                if matching_entry.refid != *refid {
                    // Yeah, too lazy to do a proper check along the whole vec, this case shouldn't happen
                    println!("{:?} - {:?}", entry, ledger[index + 1]);
                    return Err(MappingError::new(String::from("We have unmatching refid, meaning either something appeared between a trade or data are broken.
                    Either way, show this error to the dev on github with your data, and we will help you. This shouldn't happen")));
                }
                index += 1;

                let first_currency = &entry.asset;
                let second_currency = &matching_entry.asset;

                let wallet_from = create_or_get_wallet(wallet_ids,wallets,first_currency,&platform,&None,&entry.balance)?;
                let wallet_to = create_or_get_wallet(wallet_ids,wallets,second_currency,&platform,&None,&matching_entry.balance)?;

                let sold_amount = - Decimal::from_str_exact(&entry.amount).map_err(|e| MappingError::new(e.to_string()))?;
                let bought_amount = Decimal::from_str_exact(&matching_entry.amount).map_err(|e| MappingError::new(e.to_string()))?;
                let first_fee = Decimal::from_str_exact(&entry.fee).map_err(|e| MappingError::new(e.to_string()))?;
                let second_fee = Decimal::from_str_exact(&matching_entry.fee).map_err(|e| MappingError::new(e.to_string()))?;
                let fee = get_fee(first_fee,second_fee)?;
                let mut fee_price: Option<Decimal> = None;
                let mut bought_price_eur : Decimal = dec!(0);
                let mut is_taxable = false;
                if second_currency == "ZEUR"{
                    fee_price = Some(sold_amount / bought_amount); // Price of in euro of the asset we sold
                    bought_price_eur = dec!(1); // Because it is euro already
                    is_taxable=true;
                }  else if first_currency == "ZEUR" {
                    fee_price = 1;
                    bought_price_eur = dec!(1); // Because it is euro already
                } else if FiatKraken::from_str(&second_currency).is_some(){
                    is_taxable=true;
                    let time = matching_entry.time;
                    if let Some(pair) = pairs.get(&(second_currency.to_string(),String::from("ZEUR"))).cloned(){
                        tokio::spawn(async move {
                            let price = get_pair_price(time,pair.to_string()).await;
                            bought_price_eur = price;
                        });
                    } 
                    else if let Some(pair) = pairs.get(&(second_currency.to_string(),String::from("XBT"))).cloned() { // We use BTC, then EUR to get the price
                        tokio::spawn(async move {
                            let price_btc = get_pair_price(time,pair.to_string()).await;
                            let price_btc_eur = get_pair_price(time,pair.to_string()).await;
                            bought_price_eur = price_btc * price_btc_eur;
                        });
                    } else {
                        return Err(MappingError::new(format!("Couldn't find price for pair {first_currency}/{second_currency}")));
                    }
                }
                
                
                else{
                    let time = entry.time;
                    // We assume the fee is always in the first currency for now
                    if let Some(pair) = pairs.get(&(first_currency.to_string(),String::from("ZEUR"))).cloned(){
                        tokio::spawn(async move {
                            let price = get_pair_price(time,pair.to_string()).await;
                            fee_price = Some(price);
                        });
                    } 
                    else if let Some(pair) = pairs.get(&(first_currency.to_string(),String::from("XBT"))).cloned() { // We use BTC, then EUR to get the price
                        tokio::spawn(async move {
                            let price_btc = get_pair_price(time,pair.to_string()).await;
                            let price_btc_eur = get_pair_price(time,pair.to_string()).await;
                            fee_price = Some(price_btc * price_btc_eur);
                        });
                    } else {
                        return Err(MappingError::new(format!("Couldn't find price for pair {first_currency}/{second_currency}")));
                    }
                    
                }
                
                // We initialize to 0, I am currently too lazy to overthink if we can use an Option or not 
                // We should only calculate pf_total_value when we are sure to have all the data we need (after parsing)
                // the cost_basis will also be calculated later.
                let pf = CurrentPortfolio{ pf_total_value: dec!(0), pf_cost_basis: dec!(0), pf_total_cost: dec!(0) };
                let tx = Transaction::Trade { tx: TransactionBase{
                    id: generate_id(),
                    fee,
                    timestamp: Utc.timestamp_opt(entry.time as i64, 0).unwrap(),
                    is_taxable,
                    fee_price
                }, from: wallet_from, to: wallet_to, sold_amount, bought_amount, bought_price_eur, pf };
                txs.push(tx);
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

// fn get_price_eur(fee_price: &'amut Option<Decimal>, pairs: &HashMap<(String, String), String>, currency: &String, time: f64){
//     if let Some(pair) = pairs.get(&(currency.to_string(),String::from("ZEUR"))).cloned(){
//         tokio::spawn(async move {
//             let price = get_pair_price(time,pair.to_string()).await;
//             *fee_price = Some(price);
//         });
//     } 
//     else if let Some(pair) = pairs.get(&(currency.to_string(),String::from("XBT"))).cloned() { // We use BTC, then EUR to get the price
//         tokio::spawn(async move {
//             let price_btc = get_pair_price(time,pair.to_string()).await;
//             let price_btc_eur = get_pair_price(time,pair.to_string()).await;
//             *fee_price = Some(price_btc * price_btc_eur);
//         });
//     }
// }

fn get_fee(first_fee: Decimal, second_fee: Decimal) -> Result<Option<Decimal>,MappingError> {
    return match (first_fee,second_fee){
        (zero1,zero2) if zero1 == zero2  && zero2 == dec!(0) => Ok(None),
        (fee, zero) if zero == dec!(0) => Ok(Some(fee)),
        (zero, fee) if zero == dec!(0) => Ok(Some(fee)),
        _ => Err(MappingError::new(String::from("Double fee on a trade is not supported")))
    };
}

async fn get_pair_price(time: f64, trading_pair: String) -> Decimal{
    let prices = fetch_specific_trade_data(time,trading_pair.clone()).await.unwrap();
    let result = prices.result.unwrap();
    let vec_prices = result.trades.get(&trading_pair).unwrap();
    let mut total = dec!(0);
    for price in vec_prices{
        total += Decimal::from_str_exact(&price.price).unwrap();
    }
    return total / Decimal::from(vec_prices.len()) ;
}


fn create_or_get_wallet(wallet_ids: &mut WalletIdMap,
    wallets: &mut HashMap<String, Wallet>,currency: &String, platform : &Platform, address : &Option<String>, balance: &String) -> Result<String,MappingError>{
    let wallet_id = wallet_ids.get(currency, platform, address);
    if let  Some(id) = wallet_id {
        return Ok(id);
    } else {
        let wallet_from: Wallet; 
        let wallet_base = WalletBase{
            id : generate_id(),
            currency: currency.clone(),
            platform: platform.clone(),
            address: None,
            owner: Owner::User,
            balance: Decimal::from_str_exact(balance).map_err(|e| MappingError::new(e.to_string()))?,
            info: None,
        };
        if FiatKraken::from_str(&currency).is_some(){
            wallet_from = Wallet::Fiat(wallet_base)
        }else{
            wallet_from = Wallet::Crypto(wallet_base)
        }
        let currency = wallet_from.get_currency();
        let wallet_id = wallet_from.get_id();
        wallet_ids.insert(currency, platform.clone(), None, wallet_id.clone());
        wallets.insert(wallet_id.clone(), wallet_from);
        return Ok(wallet_id);
    } 
}

/* Map the result from API to a HashMap that gives the asset pair name from a pair of asset */
pub fn map_asset_pairs(pairs: HashMap<String, AssetPair>) -> KrakenPairs{
    let mut hashmap = HashMap::new();

    for (_key,value) in pairs{
        hashmap.insert((value.base,value.quote), value.altname);
    }

    return KrakenPairs(hashmap);
}

#[derive(Serialize,Deserialize,Debug)]
pub struct KrakenPairs(HashMap<(String, String), String>);

impl KrakenPairs{
    pub fn get(self) -> HashMap<(String, String), String>{
        return self.0;
    }
}

// Get more info with https://api.kraken.com/0/public/Assets
pub enum FiatKraken{
    ZUSD,
    ZEUR,
    ZCAD,
    ZAUD,
    ZGBP,
    CHF,
    ZJPY
}

impl FiatKraken {
    pub fn from_str(s: &str) -> Option<FiatKraken> {
        match s {
            "ZUSD" => Some(FiatKraken::ZUSD),
            "ZEUR" => Some(FiatKraken::ZEUR),
            "ZCAD" => Some(FiatKraken::ZCAD),
            "ZAUD" => Some(FiatKraken::ZAUD),
            "ZGBP" => Some(FiatKraken::ZGBP),
            "CHF" => Some(FiatKraken::CHF),
            "ZJPY" => Some(FiatKraken::ZJPY),
            _ => None,
        }
    }
}





#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_fee(){
        let zero1 = dec!(0);
        let zero2 = dec!(0);
        let fee1 = dec!(1);
        let fee2 = dec!(2);

        assert_eq!(get_fee(zero1,zero2).unwrap(),None);
        assert_eq!(get_fee(fee1,zero2).unwrap(),Some(fee1));
        assert_eq!(get_fee(zero1,fee2).unwrap(),Some(fee2));
        assert!(get_fee(fee1,fee2).is_err());
    }
}