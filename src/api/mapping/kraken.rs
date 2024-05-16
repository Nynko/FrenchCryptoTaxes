use chrono::{TimeZone, Utc};
use hashbrown::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        fetch_specific_trade_data, AssetPair, Deposit, EntryType, LedgerHistory, TradeInfo,
        Withdrawal,
    },
    errors::MappingError,
    structs::{
        transaction::Taxable,
        wallet::{Owner, Platform, WalletBase},
        wallet_manager::{self, WalletManager},
        GlobalCostBasis, TradeType, Transaction, TransactionBase, Wallet, WalletIdMap,
    },
    utils::{f64_to_datetime_utc, generate_id},
};

/* This function take existing currencies, wallets and Transactions and add the new elements  */
#[tokio::main] // Async for calling API for getting asset price
pub async fn create_kraken_txs(
    wallet_manager: &mut WalletManager,
    txs: &mut Vec<Transaction>,
    ledger: Vec<LedgerHistory>,
    trades: HashMap<String, TradeInfo>,
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
                let trade = trades.get(refid).unwrap();

                let (selling, buying) = get_trade_in_order(entry, matching_entry)?;

                let sold_currency = &selling.asset;
                let buy_currency = &buying.asset;

                let pair: (&String, &String);
                if trade.r#type == "sell" {
                    // Exemple BTC/EUR --> if buy: order will be (selling) EUR then (buy) BTC but we want the pair BTC/EUR (XBT/ZEUR)
                    pair = (sold_currency, buy_currency);
                } else {
                    pair = (buy_currency, sold_currency);
                }

                let wallet_from = create_or_get_wallet(
                    wallet_manager,
                    sold_currency,
                    &platform,
                    &None,
                    &selling.balance,
                )?;
                let wallet_to = create_or_get_wallet(
                    wallet_manager,
                    buy_currency,
                    &platform,
                    &None,
                    &buying.balance,
                )?;

                let sold_amount = Decimal::from_str_exact(&selling.amount)
                    .map_err(|e| MappingError::new(e.to_string()))?
                    .abs();
                let bought_amount = Decimal::from_str_exact(&buying.amount)
                    .map_err(|e| MappingError::new(e.to_string()))?
                    .abs();
                let first_fee = Decimal::from_str_exact(&selling.fee)
                    .map_err(|e| MappingError::new(e.to_string()))?;
                let second_fee = Decimal::from_str_exact(&buying.fee)
                    .map_err(|e| MappingError::new(e.to_string()))?;
                let fee = get_fee(first_fee, second_fee)?;
                let mut fee_price: Option<Decimal> = None;
                let mut taxable: Option<Taxable> = None;
                let mut trade_type = TradeType::CryptoToCrypto;
                if buy_currency == "ZEUR" {
                    fee_price = Some(sold_amount / bought_amount); // Price of in euro of the asset we sold
                    let bought_price_eur = dec!(1); // Because it is euro already
                    let is_taxable = true;
                    taxable = Some(Taxable {
                        is_taxable,
                        price_eur: bought_price_eur,
                        pf_total_value: dec!(0),
                        is_pf_total_calculated: false,
                    });
                    trade_type = TradeType::CryptoToFiat;
                } else if sold_currency == "ZEUR" {
                    fee_price = Some(dec!(1));
                    trade_type = TradeType::FiatToCrypto {
                        local_cost_basis: sold_amount,
                    };
                } else if FiatKraken::from_str(&buy_currency).is_some() {
                    let is_taxable = true;
                    trade_type = TradeType::CryptoToFiat;
                    let time = buying.time;
                    if let Some(pair) = pairs
                        .get(&(pair.0.to_string(), String::from("ZEUR")))
                        .cloned()
                    {
                        let price = get_pair_price(time, pair.to_string()).await;
                        let bought_price_eur = price;
                        taxable = Some(Taxable {
                            is_taxable,
                            price_eur: bought_price_eur,
                            pf_total_value: dec!(0),
                            is_pf_total_calculated: false,
                        });
                    } else if let Some(pair) = pairs
                        .get(&(pair.0.to_string(), String::from("XBT")))
                        .cloned()
                    // We use BTC, then EUR to get the price
                    {
                        let price_btc = get_pair_price(time, pair.to_string()).await;
                        let price_btc_eur = get_pair_price(time, pair.to_string()).await;
                        let bought_price_eur = price_btc * price_btc_eur;
                        taxable = Some(Taxable {
                            is_taxable,
                            price_eur: bought_price_eur,
                            pf_total_value: dec!(0),
                            is_pf_total_calculated: false,
                        });
                    } else {
                        return Err(MappingError::new(format!(
                            "Couldn't find price for pair {sold_currency}/{buy_currency}"
                        )));
                    }
                } else if FiatKraken::from_str(&sold_currency).is_some() {
                    let time = buying.time;
                    if let Some(pair) = pairs
                        .get(&(pair.0.to_string(), String::from("ZEUR")))
                        .cloned()
                    {
                        let price = get_pair_price(time, pair.to_string()).await;
                        trade_type = TradeType::FiatToCrypto {
                            local_cost_basis: price,
                        };
                    } else if let Some(pair) = pairs
                        .get(&(pair.0.to_string(), String::from("XBT")))
                        .cloned()
                    // We use BTC, then EUR to get the price
                    {
                        let price_btc = get_pair_price(time, pair.to_string()).await;
                        let price_btc_eur = get_pair_price(time, pair.to_string()).await;
                        let bought_price_eur = price_btc * price_btc_eur;
                        trade_type = TradeType::FiatToCrypto {
                            local_cost_basis: bought_price_eur,
                        };
                    } else {
                        return Err(MappingError::new(format!(
                            "Couldn't find price for pair {sold_currency}/{buy_currency}"
                        )));
                    }
                } else {
                    let time = selling.time;
                    // We assume the fee is always in the first currency for now
                    if let Some(pair) = pairs
                        .get(&(sold_currency.to_string(), String::from("ZEUR")))
                        .cloned()
                    {
                        let price = get_pair_price(time, pair.to_string()).await;
                        fee_price = Some(price);
                    } else if let Some(pair) = pairs
                        .get(&(sold_currency.to_string(), String::from("XBT")))
                        .cloned()
                    {
                        // We use BTC, then EUR to get the price

                        let price_btc = get_pair_price(time, pair.to_string()).await;
                        let price_btc_eur = get_pair_price(time, pair.to_string()).await;
                        fee_price = Some(price_btc * price_btc_eur);
                    } else {
                        return Err(MappingError::new(format!(
                            "Couldn't find price for pair {sold_currency}/{buy_currency}"
                        )));
                    }
                }

                // We initialize to 0, I am currently too lazy to overthink if we can use an Option or not
                // The cost_basis will be calculated later.
                let cost_basis = GlobalCostBasis {
                    pf_cost_basis: dec!(0),
                    pf_total_cost: dec!(0),
                };
                let tx = Transaction::Trade {
                    tx: TransactionBase {
                        id: refid.to_string(),
                        fee,
                        timestamp: f64_to_datetime_utc(selling.time).unwrap(),
                        fee_price,
                    },
                    from: wallet_from,
                    to: wallet_to,
                    exchange_pair: Some((pair.0.clone(), pair.1.clone())),
                    sold_amount,
                    bought_amount,
                    trade_type,
                    taxable,
                    cost_basis,
                };
                txs.push(tx);
            }
            // EntryType::Transfer => { // Mostly staking to spot and spot to staking, can apply but no need for now
            // Only useful for having more notion of what happened to determine what we did.
            //     match entry.subtype{
            //         "stakingtospot" => {},
            //         _ => (),
            //     }

            // },
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

fn get_trade_in_order<'a>(
    entry: &'a LedgerHistory,
    matching_entry: &'a LedgerHistory,
) -> Result<(&'a LedgerHistory, &'a LedgerHistory), MappingError> {
    let sold_amount =
        Decimal::from_str_exact(&entry.amount).map_err(|e| MappingError::new(e.to_string()))?;
    if sold_amount < dec!(0) {
        return Ok((entry, matching_entry));
    } else {
        return Ok((matching_entry, entry));
    }
}

fn get_fee(first_fee: Decimal, second_fee: Decimal) -> Result<Option<Decimal>, MappingError> {
    return match (first_fee, second_fee) {
        (zero1, zero2) if zero1 == zero2 && zero2 == dec!(0) => Ok(None),
        (fee, zero) if zero == dec!(0) => Ok(Some(fee)),
        (zero, fee) if zero == dec!(0) => Ok(Some(fee)),
        _ => Err(MappingError::new(String::from(
            "Double fee on a trade is not supported",
        ))),
    };
}

async fn get_pair_price(time: f64, trading_pair: String) -> Decimal {
    let prices = fetch_specific_trade_data(time, trading_pair.clone())
        .await
        .unwrap();
    let result = prices.result.unwrap();
    let pair_key = result.trades.keys().next().unwrap(); // Should only contain one value
    let vec_prices = result.trades.get(pair_key).unwrap();
    let mut total = dec!(0);
    for price in vec_prices {
        total += Decimal::from_str_exact(&price.price).unwrap();
    }
    return total / Decimal::from(vec_prices.len());
}

fn create_or_get_wallet(
    wallet_manager: &mut WalletManager,
    currency: &String,
    platform: &Platform,
    address: &Option<String>,
    balance: &String,
) -> Result<String, MappingError> {
    let wallet_ids = &mut wallet_manager.wallet_ids;
    let wallets = &mut wallet_manager.wallets;
    let wallet_id = wallet_ids.get(currency, platform, address);
    if let Some(id) = wallet_id {
        return Ok(id);
    } else {
        let wallet_from: Wallet;
        let wallet_base = WalletBase {
            id: generate_id(),
            currency: currency.clone(),
            platform: platform.clone(),
            address: None,
            owner: Owner::User,
            balance: Decimal::from_str_exact(balance)
                .map_err(|e| MappingError::new(e.to_string()))?,
            info: None,
        };
        if FiatKraken::from_str(&currency).is_some() {
            wallet_from = Wallet::Fiat(wallet_base)
        } else {
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
pub fn map_asset_pairs(pairs: HashMap<String, AssetPair>) -> KrakenPairs {
    let mut hashmap = HashMap::new();

    for (_key, value) in pairs {
        hashmap.insert((value.base, value.quote), value.altname);
    }

    return KrakenPairs(hashmap);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KrakenPairs(HashMap<(String, String), String>);

impl KrakenPairs {
    pub fn get(self) -> HashMap<(String, String), String> {
        return self.0;
    }
}

// Get more info with https://api.kraken.com/0/public/Assets
pub enum FiatKraken {
    ZUSD,
    ZEUR,
    ZCAD,
    ZAUD,
    ZGBP,
    CHF,
    ZJPY,
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
    fn test_get_fee() {
        let zero1 = dec!(0);
        let zero2 = dec!(0);
        let fee1 = dec!(1);
        let fee2 = dec!(2);

        assert_eq!(get_fee(zero1, zero2).unwrap(), None);
        assert_eq!(get_fee(fee1, zero2).unwrap(), Some(fee1));
        assert_eq!(get_fee(zero1, fee2).unwrap(), Some(fee2));
        assert!(get_fee(fee1, fee2).is_err());
    }
}
