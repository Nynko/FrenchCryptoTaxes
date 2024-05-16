use crate::structs::{GlobalCostBasis, Taxable, TradeType, Transaction, TransactionBase};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/* Calculate the french "plus-value" to fill the form 2086

plus_value =  prix_cession - (acquisition_pf_net * prix_cession / valeur_pf )

The transaction must be taxable, otherwise it will panic !
*/
pub fn calculate_tax_gains(tx: Transaction) -> Decimal {
    match tx {
        Transaction::Transfer {
            amount,
            tx,
            taxable,
            cost_basis: pf,
            ..
        } => {
            let taxable = taxable.unwrap();
            let price = taxable.price_eur;
            let pf_total_value = taxable.pf_total_value;
            let sell_price: Decimal = Decimal::from(amount) * price;
            return _calculate_tax(sell_price, &pf, pf_total_value);
        }
        Transaction::Trade {
            bought_amount,
            tx,
            taxable,
            cost_basis: pf,
            ..
        } => {
            let taxable = taxable.unwrap();
            let bought_price_eur = taxable.price_eur;
            let pf_total_value = taxable.pf_total_value;
            let sell_price: Decimal = Decimal::from(bought_amount) * bought_price_eur;
            return _calculate_tax(sell_price, &pf, pf_total_value);
        }
        _ => dec!(0),
    }
}

pub fn _calculate_tax(
    sell_price: Decimal,
    pf: &GlobalCostBasis,
    pf_total_value: Decimal,
) -> Decimal {
    return sell_price - calculate_weigted_price(sell_price, pf.pf_cost_basis, pf_total_value);
}

/* (acquisition_pf_net * prix_cession / valeur_pf ) */
pub fn calculate_weigted_price(
    sell_price: Decimal,
    current_cost_basis: Decimal,
    pf_total_value: Decimal,
) -> Decimal {
    return current_cost_basis * sell_price / pf_total_value;
}

/* Calculate the cost_basis, here "acquisition_pf_net" */
pub fn calculate_cost_basis(tx: &mut Transaction, current_pf: GlobalCostBasis) -> GlobalCostBasis {
    match tx {
        Transaction::Transfer {
            tx,
            amount,
            taxable,
            cost_basis: pf,
            ..
        } => {
            pf.pf_cost_basis = current_pf.pf_cost_basis;
            pf.pf_total_cost = current_pf.pf_total_cost;
            return calculate_new_cost_basis(tx, taxable, &pf, *amount);
        }
        Transaction::Trade {
            tx,
            bought_amount,
            trade_type,
            taxable,
            cost_basis: pf,
            ..
        } => {
            let added_cost = match trade_type {
                TradeType::FiatToCrypto { local_cost_basis } => *local_cost_basis,
                _ => dec!(0),
            };
            pf.pf_cost_basis = current_pf.pf_cost_basis + added_cost;
            pf.pf_total_cost = current_pf.pf_total_cost + added_cost;
            return calculate_new_cost_basis(tx, taxable, &pf, *bought_amount);
        }
        _ => current_pf, // ignoring the fiat deposit and withdrawal as they don't change the cost basis, they are here for accounting
    }
}

/* Calculate new Portfolio: if the transaction is taxable the new cost basis will change, otherwise only the fee might change it */
fn calculate_new_cost_basis(
    tx: &TransactionBase,
    taxable: &Option<Taxable>,
    current_pf: &GlobalCostBasis,
    amount: Decimal,
) -> GlobalCostBasis {
    let current_cost_basis = current_pf.pf_cost_basis;
    let current_total_cost = current_pf.pf_total_cost;

    let fee = if tx.fee.is_some() && tx.fee_price.is_some() {
        tx.fee.unwrap() * tx.fee_price.unwrap()
    } else {
        dec!(0.00)
    };
    let mut cost_basis_adjustment: Decimal = dec!(0.00);
    let mut pf_value_adjustment: Decimal = fee;
    if let Some(taxable) = taxable {
        if taxable.is_taxable {
            // Selling of Crypto - Taxable event
            let sell_price: Decimal = Decimal::from(amount) * taxable.price_eur;
            let weigted_price =
                calculate_weigted_price(sell_price, current_cost_basis, taxable.pf_total_value);
            cost_basis_adjustment = weigted_price;
            pf_value_adjustment = sell_price + fee
        }
    }

    return GlobalCostBasis {
        pf_cost_basis: current_cost_basis - cost_basis_adjustment + fee,
        pf_total_cost: current_total_cost + fee,
    };
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::structs::transaction::Taxable;
    use crate::structs::{Owner, Platform, Wallet, WalletBase};

    use super::*;

    use super::calculate_cost_basis;

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
        let platform = "Binance";
        let (btc_wallet, eur_wallet, eth_wallet) = create_wallets();

        let from = btc_wallet;

        let price_eur_btc = dec!(64000.02);
        let fee = dec!(0.001);
        let fee_eur = fee * price_eur_btc;

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Transfer {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: Some(fee),
                fee_price: Some(price_eur_btc),
                timestamp: Utc::now(),
            },
            from: from.get_id(),
            to: from.get_id(),
            amount: dec!(1),
            taxable: None,
            cost_basis: init_pf,
        };

        let new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(500) + fee_eur);
        assert_eq!(new_pf.pf_cost_basis, dec!(500) + fee_eur);
    }

    #[test]
    fn simple_trades() {
        let current_pf = get_pf(dec!(18000), dec!(18000));
        let platform: &str = "Binance";
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(5),
            bought_amount: dec!(20000),
            trade_type: TradeType::CryptoToFiat,
            taxable: Some(Taxable {
                is_taxable: true,
                price_eur: dec!(1),
                pf_total_value: dec!(32000),
                is_pf_total_calculated: true,
            }),
            cost_basis: init_pf,
        };

        let new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(18000));
        assert_eq!(new_pf.pf_cost_basis, dec!(18000) - dec!(11250));

        let gains = calculate_tax_gains(tx);
        assert_eq!(gains, dec!(8750));
    }

    #[test]
    fn simple_two_trades() {
        // let current_pf = get_pf(dec!(1000), dec!(1000));
        let platform: &str = "Binance";
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx0 = Transaction::Trade {
            tx: TransactionBase {
                id: "test0".to_string(),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
            },
            from: eur_wallet.get_id(),
            to: btc_wallet.get_id(),
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1),
            bought_amount: dec!(450),
            trade_type: TradeType::FiatToCrypto {
                local_cost_basis: dec!(1000),
            },
            taxable: None,
            cost_basis: init_pf.clone(),
        };

        let mut current_pf = calculate_cost_basis(&mut tx0, init_pf.clone());

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1),
            bought_amount: dec!(450),
            trade_type: TradeType::CryptoToFiat,
            taxable: Some(Taxable {
                is_taxable: true,
                price_eur: dec!(1),
                pf_total_value: dec!(1200),
                is_pf_total_calculated: true,
            }),
            cost_basis: init_pf.clone(),
        };

        let mut new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(1000));
        assert_eq!(new_pf.pf_cost_basis, dec!(1000) - dec!(375));

        let gains = calculate_tax_gains(tx);
        assert_eq!(gains, dec!(75));

        // Price update
        let init_pf2 = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: "test2".to_string(),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
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
            cost_basis: init_pf2,
        };

        let new_pf2 = calculate_cost_basis(&mut tx2, new_pf);

        assert_eq!(new_pf2.pf_total_cost, dec!(1000));

        assert_eq!(new_pf2.pf_cost_basis, dec!(0));

        let gains = calculate_tax_gains(tx2);
        assert_eq!(gains, dec!(675));
    }
}
