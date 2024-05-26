use chrono::Utc;
use rust_decimal_macros::dec;
use serde_json::to_string;

use crate::{
    functions::{calculate_full_cost_basis, calculate_tax_gains},
    structs::{
        portfolio_manager::{self, PortfolioManager}, wallet_manager::{self, WalletManager}, GlobalCostBasis, Owner, Persistable, Platform, Taxable, TradeType, Transaction, TransactionBase, Wallet, WalletBase, WalletSnapshot
    },
};

#[test]
fn simple_two_trades() {
    let init_pf = GlobalCostBasis {
        pf_cost_basis: dec!(0),
        pf_total_cost: dec!(0),
    };

    let tx0 = Transaction::Trade {
        tx: TransactionBase {
            id: "test0".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "eur".to_string(),
            fee: None,
            pre_tx_balance: dec!(1000),
            price_eur: Some(dec!(1)),
        },
        to: WalletSnapshot {
            id: "btc".to_string(),
            fee: None,
            pre_tx_balance: dec!(0),
            price_eur: Some(dec!(1)),
        },
        exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
        sold_amount: dec!(1000),
        bought_amount: dec!(2),
        trade_type: TradeType::FiatToCrypto {
            local_cost_basis: dec!(1000),
        },
        taxable: None,
        cost_basis: init_pf.clone(),
    };

    let tx1 = Transaction::Trade {
        tx: TransactionBase {
            id: "test".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            pre_tx_balance: dec!(2),
            fee: None,
            price_eur: Some(dec!(600)),
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            pre_tx_balance: dec!(0),
            fee: None,
            price_eur: Some(dec!(1)),
        },
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

    let tx2 = Transaction::Trade {
        tx: TransactionBase {
            id: "test2".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            pre_tx_balance: dec!(1),
            fee: None,
            price_eur: Some(dec!(1300)),
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            pre_tx_balance: dec!(450),
            fee: None,
            price_eur: Some(dec!(1)),
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
        cost_basis: init_pf.clone(),
    };

    let mut transactions = vec![tx0, tx1, tx2];

    let mut wallet_manager =
        WalletManager::new(Some(".data_test/simple_trade_wallet".to_string())).unwrap();

    wallet_manager.wallets.insert(
        "btc".to_string(),
        Wallet::Crypto(WalletBase {
            id: "btc".to_string(),
            currency: "BTC".to_string(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: dec!(0),
            info: None,
        }),
    );

    wallet_manager.wallets.insert(
        "eur".to_string(),
        Wallet::Fiat(WalletBase {
            id: "eur".to_string(),
            currency: "EUR".to_string(),
            platform: Platform::Binance,
            address: None,
            owner: Owner::User,
            balance: dec!(0),
            info: None,
        }),
    );

    let mut portfolio_manager =
        PortfolioManager::new(Some(".data_test/simple_trade".to_string())).unwrap();

    portfolio_manager
        .calculate_portfolio_history_and_update_tx(&mut transactions, &wallet_manager.wallets)
        .unwrap();

    assert_eq!(transactions[1].get_taxable().as_ref().unwrap().pf_total_value, dec!(1200));
    assert_eq!(transactions[2].get_taxable().as_ref().unwrap().pf_total_value, dec!(1300));

    calculate_full_cost_basis(&mut transactions);
    let gains = calculate_tax_gains(&transactions[1]);
    assert_eq!(gains, dec!(75));
    let gains = calculate_tax_gains(&transactions[2]);
    assert_eq!(gains, dec!(675));

    let _ = portfolio_manager.delete();
}
