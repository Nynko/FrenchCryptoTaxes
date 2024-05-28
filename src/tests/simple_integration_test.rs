use chrono::Utc;
use rust_decimal_macros::dec;

use crate::{
    functions::calculate_tax_gains,
    structs::{
        portfolio_manager::PortfolioManager, wallet_manager::WalletManager, GlobalCostBasisManager, Owner, Persistable, Platform, TradeType, Transaction, TransactionBase, Wallet, WalletBase, WalletSnapshot
    },
};

#[test]
fn simple_two_trades() {
    let tx0 = Transaction::Trade {
        tx: TransactionBase {
            id: "test0".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "eur".to_string(),
            fee: None,
            pre_tx_balance: dec!(1000),
            price_eur: dec!(1),
        },
        to: WalletSnapshot {
            id: "btc".to_string(),
            fee: None,
            pre_tx_balance: dec!(0),
            price_eur: dec!(0), // No need for that 
        },
        exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
        sold_amount: dec!(1000),
        bought_amount: dec!(3),
        trade_type: TradeType::FiatToCrypto {
            local_cost_basis: dec!(1000),
        },
    };

    let tx1 = Transaction::Trade {
        tx: TransactionBase {
            id: "test".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            pre_tx_balance: dec!(3),
            fee: None,
            price_eur: dec!(400),
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            pre_tx_balance: dec!(0),
            fee: None,
            price_eur: dec!(1),
        },
        exchange_pair: Some(("BTC".to_string(), "EUR".to_uppercase())),
        sold_amount: dec!(1.125),
        bought_amount: dec!(450),
        trade_type: TradeType::CryptoToFiat,
    };

    let tx2 = Transaction::Trade {
        tx: TransactionBase {
            id: "test2".to_string(),
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            pre_tx_balance: dec!(1.875),
            fee: None,
            price_eur: dec!(693.33333333333333333333333333),
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            pre_tx_balance: dec!(450),
            fee: None,
            price_eur: dec!(1),
        },
        exchange_pair: None,
        sold_amount: dec!(1.875),
        bought_amount: dec!(1300),
        trade_type: TradeType::CryptoToFiat,
    };

    let transactions = vec![tx0, tx1, tx2];

    let mut wallet_manager =
        WalletManager::new_non_persistent().unwrap();

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
        PortfolioManager::new_non_persistent().unwrap();

    portfolio_manager
        .calculate_portfolio_history(&transactions, &wallet_manager.wallets)
        .unwrap();

    let tx_id_1 = transactions[1].get_id();
    let tx_id_2 = transactions[2].get_id();

    assert_eq!(portfolio_manager.portfolio_history.get(tx_id_1).unwrap().pf_total_value, dec!(1200));
    assert_eq!(portfolio_manager.portfolio_history.get(tx_id_2).unwrap().pf_total_value, dec!(1300));


    let mut cost_basis_manager = GlobalCostBasisManager::new_non_persistent().unwrap();
    cost_basis_manager.calculate_full_cost_basis(&transactions, &portfolio_manager.portfolio_history);
    assert_eq!(cost_basis_manager.global_cost_basis_history.get(tx_id_1).unwrap().pf_total_cost, dec!(1000));
    assert_eq!(cost_basis_manager.global_cost_basis_history.get(tx_id_1).unwrap().pf_cost_basis, dec!(1000));
    assert_eq!(cost_basis_manager.global_cost_basis_history.get(tx_id_2).unwrap().pf_total_cost, dec!(1000));
    assert_eq!(cost_basis_manager.global_cost_basis_history.get(tx_id_2).unwrap().pf_cost_basis, dec!(1000) - dec!(375));
    let gains = calculate_tax_gains(&transactions[1],portfolio_manager.portfolio_history.get(tx_id_1).unwrap(),cost_basis_manager.global_cost_basis_history.get(tx_id_1).unwrap());
    assert_eq!(gains, dec!(75));
    let gains = calculate_tax_gains(&transactions[2],portfolio_manager.portfolio_history.get(tx_id_2).unwrap(),cost_basis_manager.global_cost_basis_history.get(tx_id_2).unwrap());
    assert_eq!(gains, dec!(675));

    let _ = portfolio_manager.delete();
}
