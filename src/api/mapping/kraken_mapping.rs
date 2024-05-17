use hashbrown::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        fetch_specific_trade_data, kraken_pairs, AssetPair, Deposit, EntryType, LedgerHistory,
        TradeInfo, Withdrawal,
    },
    errors::{ApiError, MappingError},
    structs::{
        transaction::Taxable,
        wallet::{Owner, Platform, WalletBase},
        wallet_manager::WalletManager,
        GlobalCostBasis, TradeType, Transaction, TransactionBase, Wallet, WalletSnapshot,
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
) -> Result<(), ApiError> {
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
                    return Err(ApiError::MappingError(MappingError::Other(String::from("We have unmatching refid, meaning either something appeared between a trade or data are broken.
                    Either way, show this error to the dev on github with your data, and we will help you. This shouldn't happen"))));
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

                let selling_amount = selling.amount.abs();

                let wallet_from = create_or_get_wallet(
                    wallet_manager,
                    sold_currency,
                    &platform,
                    &None,
                    selling.balance,
                    selling_amount,
                    selling.fee,
                )?;

                if refid == "TLK2JQ-B7ZFH-JOY3I5" {
                    println!("HEEEEERRRREEEE {:?}", wallet_from);
                }
                let wallet_to = create_or_get_wallet(
                    wallet_manager,
                    buy_currency,
                    &platform,
                    &None,
                    buying.balance,
                    dec!(0),
                    buying.fee,
                )?;

                let time = entry.time;
                let fee_and_entry = get_fee(selling, buying)?;
                let mut fee = None;
                let mut fee_price: Option<Decimal> = None;

                // Handle the fee and fee price
                if let Some((fee_value, entry_fee, other_entry)) = fee_and_entry {
                    fee = Some(fee_value);
                    // Get fee_price
                    if entry_fee.asset == "ZEUR" {
                        fee_price = Some(dec!(1));
                    } else if other_entry.asset == "ZEUR" {
                        // We can guess the price by using the amounts
                        fee_price = Some(other_entry.amount / entry.amount); // Price of in euro of the asset we sold
                    } else {
                        // We get the price
                        let price =
                            get_currency_price(time.to_string(), entry_fee.asset.to_string())
                                .await?;
                        fee_price = Some(price);
                    }
                }

                let mut taxable: Option<Taxable> = None;
                let mut trade_type = TradeType::CryptoToCrypto;

                // Handle taxable or not
                if let Some(fiat) = FiatKraken::from_str(&buy_currency) {
                    trade_type = TradeType::CryptoToFiat;
                    let bought_price_eur: Decimal;
                    if fiat.is_eur() {
                        bought_price_eur = dec!(1); // Because it is euro already
                    } else {
                        // Need to get the price
                        bought_price_eur =
                            get_currency_price(time.to_string(), sold_currency.to_string())
                                .await?;
                    }
                    let is_taxable = true;
                    taxable = Some(Taxable {
                        is_taxable,
                        price_eur: bought_price_eur,
                        pf_total_value: dec!(0),
                        is_pf_total_calculated: false,
                    });
                } else if let Some(fiat) = FiatKraken::from_str(&sold_currency) {
                    let price_sold_currency: Decimal;
                    if fiat.is_eur() {
                        price_sold_currency = dec!(1);
                    } else if selling.fee != dec!(0) {
                        price_sold_currency = fee_price.unwrap();
                    } else {
                        let time = buying.time;
                        price_sold_currency =
                            get_currency_price(time.to_string(), sold_currency.to_string())
                                .await?;
                    }
                    trade_type = TradeType::FiatToCrypto {
                        local_cost_basis: price_sold_currency * selling_amount,
                    };
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
                    sold_amount: selling_amount,
                    bought_amount: buying.amount,
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
            EntryType::Deposit => todo!(),
            EntryType::Withdrawal => todo!(),
            // EntryType::Staking => todo!(),
            // EntryType::Reward => todo!(),
            _ => (),
        }
        index += 1;
    }
    Ok(())
}

fn get_trade_in_order<'a>(
    entry: &'a LedgerHistory,
    matching_entry: &'a LedgerHistory,
) -> Result<(&'a LedgerHistory, &'a LedgerHistory), ApiError> {
    if entry.amount.clone() < dec!(0) {
        return Ok((entry, matching_entry));
    } else {
        return Ok((matching_entry, entry));
    }
}

fn get_fee<'a>(
    selling: &'a LedgerHistory,
    buying: &'a LedgerHistory,
) -> Result<Option<(Decimal, &'a LedgerHistory, &'a LedgerHistory)>, ApiError> {
    return match (selling.fee, buying.fee) {
        (zero1, zero2) if zero1 == zero2 && zero2 == dec!(0) => Ok(None),
        (fee, zero) if zero == dec!(0) => Ok(Some((selling.fee, selling, buying))),
        (zero, fee) if zero == dec!(0) => Ok(Some((buying.fee, buying, selling))),
        _ => Err(ApiError::MappingError(MappingError::Other(String::from(
            "Double fee on a trade is not supported",
        )))),
    };
}

pub async fn get_currency_price(time: String, currency: String) -> Result<Decimal, ApiError> {
    let sanitized_currency = sanitize_currency(currency);
    let pairs = kraken_pairs().unwrap().0;
    if let Some(pair) = pairs
        .get(&(sanitized_currency.to_string(), String::from("ZEUR")))
        .cloned()
    {
        let price = get_pair_price(time, pair.to_string()).await;
        Ok(price)
    } else if let Some(pair) = pairs
        .get(&(sanitized_currency.to_string(), String::from("XBT")))
        .cloned()
    // We use BTC, then EUR to get the price
    {
        let price_btc = get_pair_price(time.to_string(), pair.to_string()).await;
        let price_btc_eur = get_pair_price(time.to_string(), "XXBTZEUR".to_string()).await;
        let bought_price_eur = price_btc * price_btc_eur;
        Ok(bought_price_eur)
    } else {
        return Err(ApiError::CouldNotFindPrice {
            pairs: vec![
                (sanitized_currency.clone(), "ZEUR".to_string()),
                (sanitized_currency, "XBT".to_string()),
            ],
        });
    }
}

async fn get_pair_price(time: String, trading_pair: String) -> Decimal {
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
    post_trade_balance: Decimal,
    amount: Decimal,
    fee: Decimal,
) -> Result<WalletSnapshot, ApiError> {
    let wallet_ids = &mut wallet_manager.wallet_ids;
    let wallets = &mut wallet_manager.wallets;
    let wallet_id = wallet_ids.get(currency, platform, address);
    if let Some(id) = wallet_id {
        return Ok(WalletSnapshot {
            id: id,
            balance: post_trade_balance + amount + fee,
            price_eur: None,
        });
    } else {
        let wallet_from: Wallet;
        let wallet_base = WalletBase {
            id: generate_id(),
            currency: currency.clone(),
            platform: platform.clone(),
            address: None,
            owner: Owner::User,
            balance: post_trade_balance, // Post trade
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
        return Ok(WalletSnapshot {
            id: wallet_id,
            balance: post_trade_balance + amount + fee, // before trade
            price_eur: None,
        });
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

pub fn sanitize_currency(currency: String) -> String {
    let currency_without_suffix = if currency.ends_with(".S") {
        currency[..currency.len() - 2].to_string()
    } else {
        currency
    };

    let sanitized = if currency_without_suffix == "ETH2" || currency_without_suffix == "ETH" {
        "XETH".to_string()
    } else {
        currency_without_suffix
    };

    return sanitized;
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

    pub fn is_eur(&self) -> bool {
        match self {
            FiatKraken::ZEUR => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_mapping() {}
}
