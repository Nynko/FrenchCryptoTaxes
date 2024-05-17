use chrono::Utc;
use rust_decimal_macros::dec;

use crate::{
    functions::{calculate_full_cost_basis, calculate_tax_gains},
    structs::{
        portfolio_manager::{self, PortfolioManager},
        GlobalCostBasis, Taxable, TradeType, Transaction, TransactionBase, WalletSnapshot,
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
            fee: None,
            fee_price: None,
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "eur".to_string(),
            balance: dec!(1000),
            price_eur: None,
        },
        to: WalletSnapshot {
            id: "btc".to_string(),
            balance: dec!(0),
            price_eur: None,
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
            fee: None,
            fee_price: None,
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            balance: dec!(2),
            price_eur: None,
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            balance: dec!(0),
            price_eur: None,
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
            fee: None,
            fee_price: None,
            timestamp: Utc::now(),
        },
        from: WalletSnapshot {
            id: "btc".to_string(),
            balance: dec!(1),
            price_eur: None,
        },
        to: WalletSnapshot {
            id: "eur".to_string(),
            balance: dec!(450),
            price_eur: None,
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

    let portfolio_manager =
        PortfolioManager::new(Some(".data_test/simple_trade".to_string())).unwrap();

    calculate_full_cost_basis(&mut transactions);
    let gains = calculate_tax_gains(&transactions[1]);
    assert_eq!(gains, dec!(75));
    let gains = calculate_tax_gains(&transactions[2]);
    assert_eq!(gains, dec!(675));

    let _ = portfolio_manager.delete();
}
