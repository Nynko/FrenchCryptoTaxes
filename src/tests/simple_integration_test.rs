use chrono::Utc;
use rust_decimal_macros::dec;
use serde_json::to_string;

use crate::{
    functions::calculate_tax_gains,
    structs::{
        portfolio_manager::{self, PortfolioManager}, wallet_manager::{self, WalletManager}, GlobalCostBasis, GlobalCostBasisManager, Owner, Persistable, Platform, TradeType, Transaction, TransactionBase, Wallet, WalletBase, WalletSnapshot
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
        cost_basis: init_pf.clone(),
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
        // taxable: Some(Taxable {
        //     is_taxable: true,
        //     price_eur: dec!(1),
        //     pf_total_value: dec!(1200),
        //     is_pf_total_calculated: true,
        // }),
        cost_basis: init_pf.clone(),
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
        // taxable: Some(Taxable {
        //     is_taxable: true,
        //     price_eur: dec!(1),
        //     pf_total_value: dec!(1300),
        //     is_pf_total_calculated: true,
        // }),
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
        .calculate_portfolio_history(&mut transactions, &wallet_manager.wallets)
        .unwrap();

    assert_eq!(portfolio_manager.portfolio_history.get(&transactions[1].get_tx_base().id).unwrap().pf_total_value, dec!(1200));
    assert_eq!(portfolio_manager.portfolio_history.get(&transactions[2].get_tx_base().id).unwrap().pf_total_value, dec!(1300));


    let mut cost_basis_manager = GlobalCostBasisManager::new(Some(".data_test/global_cost_basis".to_string())).unwrap();
    cost_basis_manager.calculate_full_cost_basis(&mut transactions, &portfolio_manager.portfolio_history);
    assert_eq!(transactions.get(1).unwrap().tmp_get_cost_basis().unwrap().pf_total_cost, dec!(1000));
    assert_eq!(transactions.get(1).unwrap().tmp_get_cost_basis().unwrap().pf_cost_basis, dec!(1000));
    assert_eq!(transactions.get(2).unwrap().tmp_get_cost_basis().unwrap().pf_total_cost, dec!(1000));
    assert_eq!(transactions.get(2).unwrap().tmp_get_cost_basis().unwrap().pf_cost_basis, dec!(1000) - dec!(375));
    let gains = calculate_tax_gains(&transactions[1],portfolio_manager.portfolio_history.get(&transactions[1].get_tx_base().id).unwrap());
    assert_eq!(gains, dec!(75));
    let gains = calculate_tax_gains(&transactions[2],portfolio_manager.portfolio_history.get(&transactions[2].get_tx_base().id).unwrap());
    assert_eq!(gains, dec!(675));

    let _ = portfolio_manager.delete();
}
