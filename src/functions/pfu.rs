use crate::structs::{CurrentPortfolio, Transaction, TransactionBase};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/* Calculate the french "plus-value" to fill the form 2086

plus_value =  prix_cession - (acquisition_pf_net * prix_cession / valeur_pf )
*/
pub fn calculate_tax_gains(tx: Transaction) -> Decimal {
    match tx {
        Transaction::Transfer {
            amount,
            price_eur,
            pf,
            ..
        } => {
            let sell_price: Decimal = Decimal::from(amount) * price_eur;
            return _calculate_tax(sell_price, &pf);
        }
        Transaction::Trade {
            bought_amount,
            bought_price_eur,
            pf,
            ..
        } => {
            let sell_price: Decimal = Decimal::from(bought_amount) * bought_price_eur;
            return _calculate_tax(sell_price, &pf);
        }
        _ => dec!(0),
    }
}

pub fn _calculate_tax(sell_price: Decimal, pf: &CurrentPortfolio) -> Decimal {
    return sell_price - calculate_weigted_price(sell_price, pf.pf_cost_basis, pf.pf_total_value);
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
pub fn calculate_cost_basis(
    tx: &mut Transaction,
    current_pf: CurrentPortfolio,
) -> CurrentPortfolio {
    match tx {
        Transaction::Transfer {
            tx,
            from: _,
            to: _,
            amount,
            price_eur,
            pf,
        } => {
            pf.pf_cost_basis = current_pf.pf_cost_basis;
            pf.pf_total_cost = current_pf.pf_total_cost;
            return calculate_new_portfolio(tx, &pf, *amount, *price_eur);
        }
        Transaction::Trade {
            tx,
            from: _,
            to: _,
            sold_amount: _,
            bought_amount,
            bought_price_eur,
            pf,
        } => {
            pf.pf_cost_basis = current_pf.pf_cost_basis;
            pf.pf_total_cost = current_pf.pf_total_cost;
            return calculate_new_portfolio(tx, &pf, *bought_amount, *bought_price_eur);
        }
        _ => current_pf, // ignoring the fiat deposit and withdrawal as they don't change the cost basis, they are here for accounting
    }
}

/* Calculate new Portfolio: if the transaction is taxable the new cost basis will change, otherwise only the fee might change it */
fn calculate_new_portfolio(
    tx: &TransactionBase,
    current_pf: &CurrentPortfolio,
    amount: u64,
    price_eur: Decimal,
) -> CurrentPortfolio {
    let current_cost_basis = current_pf.pf_cost_basis;
    let current_total_cost = current_pf.pf_total_cost;
    let pf_total_value = current_pf.pf_total_value;

    let fee = if tx.fee.is_some() && tx.fee_price.is_some() {
        tx.fee.unwrap() * tx.fee_price.unwrap()
    } else {
        dec!(0.00)
    };
    let mut cost_basis_adjustment: Decimal = dec!(0.00);
    let mut pf_value_adjustment: Decimal = fee;
    if tx.is_taxable {
        // Selling of Crypto - Taxable event
        let sell_price: Decimal = Decimal::from(amount) * price_eur;
        let weigted_price =
            calculate_weigted_price(sell_price, current_cost_basis, pf_total_value);
        cost_basis_adjustment = weigted_price;
        pf_value_adjustment = sell_price + fee
    }

    return CurrentPortfolio {
        pf_cost_basis: current_cost_basis - cost_basis_adjustment + fee,
        pf_total_cost: current_total_cost + fee,
        pf_total_value: pf_total_value - pf_value_adjustment,
    };
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::structs::{Currency, Owner, Platform, Wallet, WalletBase};

    use super::*;

    use super::calculate_cost_basis;

    fn get_pf(cost_basis: Decimal, total_cost: Decimal, total_value: Decimal) -> CurrentPortfolio {
        return CurrentPortfolio {
            pf_cost_basis: cost_basis,
            pf_total_cost: total_cost,
            pf_total_value: total_value,
        };
    }
    fn create_currencies() -> (Currency, Currency, Currency) {
        let btc_currency = Currency {
            id: "test".to_string(),
            name: "bitcoin".to_string(),
            symbol: "BTC".to_string(),
            decimals: 8,
        };

        let eur_currency = Currency {
            id: "eur".to_string(),
            name: "euro".to_string(),
            symbol: "EUR".to_string(),
            decimals: 2,
        };

        let eth_currency = Currency {
            id: "test2".to_string(),
            name: "ethereum".to_string(),
            symbol: "ETH".to_string(),
            decimals: 18,
        };

        (btc_currency, eur_currency, eth_currency)
    }

    fn create_wallets(
        btc_currency: &Currency,
        eur_currency: &Currency,
        eth_currency: &Currency,
    ) -> (Wallet, Wallet, Wallet) {
        let btc = Wallet::Crypto(WalletBase {
            id: String::from("btc"),
            currency_id: btc_currency.id.clone(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: 0,
            info: None,
        });

        let eur = Wallet::Fiat(WalletBase {
            id: String::from("eur"),
            currency_id: eth_currency.id.clone(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: 0,
            info: None,
        });

        let eth = Wallet::Crypto(WalletBase {
            id: String::from("eth"),
            currency_id: btc_currency.id.clone(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: 0,
            info: None,
        });

        (btc, eur, eth)
    }

    #[test]
    fn simple_transfer_with_fee() {
        let current_pf = get_pf(dec!(500.00), dec!(500.00), dec!(1000));
        let (btc, eur, eth) = create_currencies();
        let platform = "Binance";
        let (btc_wallet, eur_wallet, eth_wallet) = create_wallets(&btc, &eur, &eth);

        let from = btc_wallet;

        let price_eur_btc = dec!(64000.02);
        let fee = dec!(0.001);
        let fee_eur = fee * price_eur_btc;

        let init_pf = CurrentPortfolio {
            pf_total_value: dec!(1000),
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Transfer {
            tx: TransactionBase {
                id: String::from("test"),
                fee: Some(fee),
                fee_price: Some(price_eur_btc),
                timestamp: Utc::now(),
                is_taxable: false,
            },
            from: from.get_id(),
            to: from.get_id(),
            amount: 1,
            price_eur: price_eur_btc,
            pf: init_pf,
        };

        let new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(500) + fee_eur);
        assert_eq!(new_pf.pf_total_value, dec!(1000) - fee_eur);
        assert_eq!(new_pf.pf_cost_basis, dec!(500) + fee_eur);
    }

    #[test]
    fn simple_trades() {
        let current_pf = get_pf(dec!(18000), dec!(18000), dec!(32000));
        let (btc, eur, eth) = create_currencies();
        let platform: &str = "Binance";
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets(&btc, &eur, &eth);

        let init_pf = CurrentPortfolio {
            pf_total_value: dec!(32000),
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: String::from("test"),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
                is_taxable: true,
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
            sold_amount: 5,
            bought_amount: 20000,
            bought_price_eur: dec!(1),
            pf: init_pf,
        };

        let new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(18000));
        assert_eq!(new_pf.pf_total_value, dec!(12000));
        assert_eq!(new_pf.pf_cost_basis, dec!(18000) - dec!(11250));

        let gains = calculate_tax_gains(tx);
        assert_eq!(gains, dec!(8750));
    }

    #[test]
    fn simple_two_trades() {
        let current_pf = get_pf(dec!(1000), dec!(1000), dec!(1200));
        let (btc, eur, eth) = create_currencies();
        let platform: &str = "Binance";
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets(&btc, &eur, &eth);

        let init_pf = CurrentPortfolio {
            pf_total_value: dec!(1200),
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: String::from("test"),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
                is_taxable: true,
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
            sold_amount: 1,
            bought_amount: 450,
            bought_price_eur: dec!(1),
            pf: init_pf,
        };

        let mut new_pf = calculate_cost_basis(&mut tx, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(1000));
        assert_eq!(new_pf.pf_total_value, dec!(1200) - dec!(450));
        assert_eq!(new_pf.pf_cost_basis, dec!(1000) - dec!(375));

        let gains = calculate_tax_gains(tx);
        assert_eq!(gains, dec!(75));

        // Price update
        let init_pf2 = CurrentPortfolio {
            pf_total_value: dec!(1300),
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: String::from("test2"),
                fee: None,
                fee_price: None,
                timestamp: Utc::now(),
                is_taxable: true,
            },
            from: btc_wallet.get_id(),
            to: eur_wallet.get_id(),
            sold_amount: 1,
            bought_amount: 1300,
            bought_price_eur: dec!(1),
            pf: init_pf2,
        };

        let new_pf2 = calculate_cost_basis(&mut tx2, new_pf);

        assert_eq!(new_pf2.pf_total_cost, dec!(1000));

        assert_eq!(new_pf2.pf_cost_basis, dec!(0));
        assert_eq!(new_pf2.pf_total_value, dec!(0));

        let gains = calculate_tax_gains(tx2);
        assert_eq!(gains, dec!(675));
    }
}
