use crate::structs::{GlobalCostBasis, Portfolio, TradeType, Transaction, TransactionId, WalletSnapshot};
use hashbrown::HashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/* Calculate the french "plus-value" to fill the form 2086

plus_value =  prix_cession - (acquisition_pf_net * prix_cession / valeur_pf )

The transaction must be taxable, otherwise it will panic !
*/
pub fn calculate_tax_gains(tx: &Transaction, portfolio: &Portfolio) -> Decimal {
    match tx {
        Transaction::Transfer {
            amount,
            cost_basis: pf,
            to,from,
            ..
        }
        | Transaction::Trade {
            sold_amount: amount,
            cost_basis: pf,
            to, from,
            ..
        } => {
            let pf_total_value = portfolio.pf_total_value;
            let sell_price: Decimal = Decimal::from(*amount) * from.price_eur;
            let fee = to.fee.unwrap_or(dec!(0)) * to.price_eur + from.fee.unwrap_or(dec!(0)) * from.price_eur;
            return _calculate_tax(sell_price, fee,&pf, pf_total_value);
        }
        _ => dec!(0),
    }
}

pub fn _calculate_tax(
    sell_price: Decimal,
    total_fee: Decimal,
    pf: &GlobalCostBasis,
    pf_total_value: Decimal,
) -> Decimal {
    return (sell_price - total_fee) - calculate_weigted_price(sell_price, pf.pf_cost_basis, pf_total_value);
}

/* (acquisition_pf_net * prix_cession / valeur_pf ) */
pub fn calculate_weigted_price(
    sell_price: Decimal,
    current_cost_basis: Decimal,
    pf_total_value: Decimal,
) -> Decimal {
    return current_cost_basis * sell_price / pf_total_value;
}

pub fn calculate_full_cost_basis(txs: &mut Vec<Transaction>, portfolios: &HashMap<TransactionId,Portfolio> ) {
    let mut global_cost_basis = GlobalCostBasis {
        pf_cost_basis: dec!(0),
        pf_total_cost: dec!(0),
    };
    for tx in txs {
        let portfolio = portfolios.get(&tx.get_tx_base().id);
        global_cost_basis = calculate_cost_basis(tx, portfolio,global_cost_basis);
    }
}

/* Calculate the cost_basis, here "acquisition_pf_net" */
pub fn calculate_cost_basis(tx: &mut Transaction, portfolio: Option<&Portfolio>, current_pf: GlobalCostBasis) -> GlobalCostBasis {
    match tx {
        Transaction::Transfer {
            to,
            from,
            amount,
            cost_basis: pf,
            ..
        } => {
            pf.pf_cost_basis = current_pf.pf_cost_basis;
            pf.pf_total_cost = current_pf.pf_total_cost;
            return calculate_new_cost_basis(to, from, portfolio, &pf, *amount);
        }
        Transaction::Trade {
            to,
            from,
            sold_amount,
            trade_type,
            cost_basis: pf,
            ..
        } => {
            let added_cost = match trade_type {
                TradeType::FiatToCrypto { local_cost_basis } => *local_cost_basis,
                _ => dec!(0),
            };
            pf.pf_cost_basis = current_pf.pf_cost_basis + added_cost;
            pf.pf_total_cost = current_pf.pf_total_cost + added_cost;
            return calculate_new_cost_basis(to, from, portfolio, &pf, *sold_amount);
        }
        _ => current_pf, // ignoring the fiat deposit and withdrawal as they don't change the cost basis, they are here for accounting
    }
}

/* Calculate new Portfolio: if the transaction is taxable the new cost basis will change, otherwise only the fee might change it */
fn calculate_new_cost_basis(
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::structs::{Owner, Platform, TransactionBase, Wallet, WalletBase, WalletSnapshot};

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
        let (btc_wallet, _eur_wallet, _eth_wallet) = create_wallets();

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
            cost_basis: init_pf,
            income: None,
        };

        let new_pf = calculate_cost_basis(&mut tx, None, current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(500) + fee_eur);
        assert_eq!(new_pf.pf_cost_basis, dec!(500) + fee_eur);
    }

    #[test]
    fn simple_trades() {
        let current_pf = get_pf(dec!(18000), dec!(18000));
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1),
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
            cost_basis: init_pf,
        };

        let portfolio = Portfolio {
            tx_id: "test".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(32000),
            is_pf_total_calculated: true,
        };

        let new_pf = calculate_cost_basis(&mut tx,Some(&portfolio), current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(18000));
        assert_eq!(new_pf.pf_cost_basis, dec!(18000) - dec!(11250));

        let gains = calculate_tax_gains(&tx, &portfolio);
        assert_eq!(gains, dec!(8750));
    }

    #[test]
    fn simple_two_trades() {
        let (btc_wallet, eur_wallet, _eth_wallet) = create_wallets();

        let init_pf = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

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
                price_eur: dec!(500),
            },
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1000),
            bought_amount: dec!(2),
            trade_type: TradeType::FiatToCrypto {
                local_cost_basis: dec!(1000),
            },
            cost_basis: init_pf.clone(),
        };

        let current_pf = calculate_cost_basis(&mut tx0, None,init_pf.clone());

        let mut tx = Transaction::Trade {
            tx: TransactionBase {
                id: "test".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(2),
                fee: None,
                price_eur: dec!(450),
            },
            to: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(0),
                fee: None,
                price_eur: dec!(1),
            },
            exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
            sold_amount: dec!(1),
            bought_amount: dec!(450),
            trade_type: TradeType::CryptoToFiat,
            cost_basis: init_pf.clone(),
        };

        let portfolio = Portfolio {
            tx_id: "test".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(1200),
            is_pf_total_calculated: true,
        };

        let new_pf = calculate_cost_basis(&mut tx, Some(&portfolio), current_pf);

        assert_eq!(new_pf.pf_total_cost, dec!(1000));
        assert_eq!(new_pf.pf_cost_basis, dec!(1000) - dec!(375));

        let gains = calculate_tax_gains(&tx, &portfolio);
        assert_eq!(gains, dec!(75));

        // Price update
        let init_pf2 = GlobalCostBasis {
            pf_cost_basis: dec!(0),
            pf_total_cost: dec!(0),
        };

        let mut tx2 = Transaction::Trade {
            tx: TransactionBase {
                id: "test2".to_string(),
                timestamp: Utc::now(),
            },
            from: WalletSnapshot {
                id: btc_wallet.get_id().to_string(),
                pre_tx_balance: dec!(1),
                fee: None,
                price_eur: dec!(1300),
            },
            to: WalletSnapshot {
                id: eur_wallet.get_id().to_string(),
                pre_tx_balance: dec!(450),
                fee: None,
                price_eur: dec!(1),
            },
            exchange_pair: None,
            sold_amount: dec!(1),
            bought_amount: dec!(1300),
            trade_type: TradeType::CryptoToFiat,
            cost_basis: init_pf2,
        };

        let portfolio2 = Portfolio {
            tx_id: "test2".to_string(),
            wallet_snaps: HashMap::new(),
            is_taxable: true,
            pf_total_value: dec!(1300),
            is_pf_total_calculated: true,
        };

        let new_pf2 = calculate_cost_basis(&mut tx2, Some(&portfolio2), new_pf);

        assert_eq!(new_pf2.pf_total_cost, dec!(1000));

        assert_eq!(new_pf2.pf_cost_basis, dec!(0));

        let gains = calculate_tax_gains(&tx2,&portfolio2);
        assert_eq!(gains, dec!(675));
    }
}
