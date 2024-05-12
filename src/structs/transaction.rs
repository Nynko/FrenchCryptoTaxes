use super::{CurrentPortfolio, Wallet};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/* A Transaction correspond to an exchange: Crypto or Fiat
A transaction can be "taxable" meaning it is either a Crypto -> Fiat transaction, or a Crypto Payment.

We want to make sure that every Transactions Crypto -> Crypto that is not towards a known wallet is defined.

Each transaction can have a cost. This cost will often correspond to the fee.
We can deduce this fee from the total amount invested and it can also be put on the form 2086.

## Transaction from same Wallet to same Wallet:
This can mean several things:
    - it can be a trade (or swap),
    - it can be a money "creation": gift/reward/interest/staking interest/ ... (deposit are not here, see below)
    - it can be a self-transaction. This can happen in the blockchain, although it won't often happen in exchanges (often we will create
        another wallet with a discriminator: Example: The platform of the Wallet is Binance, and the Currency BTC: We can have a Wallet without discriminator (main)
        and a wallet with a "earn" discriminator which describe Binance earn).


Deposit: should never be from the same wallet, there is always an "external" wallet
         we represent that by a wallet that you own (like your bank acc) or that you don't own
         if it comes from the same wallet, it means it is an "income" (either a gift, reward...): money creation from nowhere basically

         Deposits are only available from Fiat Wallet
*/

type WalletId = String;

/* We only have two types of transactions here:

A simple fee would be a transfer transaction to a wallet not owned by the user */
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    // Transfer can be a "local" non taxable transfer, it can be a taxable transfer to external entity
    Transfer {
        tx: TransactionBase,
        from: WalletId,
        to: WalletId,
        amount: u64,
        price_eur: Decimal,
        pf: CurrentPortfolio,
    },
    // Trade can be a Crypto/Crypto non taxable trade, or taxable sold of Crypto, or non taxable event: buying crypto
    Trade {
        tx: TransactionBase,
        from: WalletId,
        to: WalletId,
        sold_amount: u64,
        bought_amount: u64,
        bought_price_eur: Decimal,
        pf: CurrentPortfolio,
    },
    Deposit {
        tx: TransactionBase,
        to: WalletId,
        amount: u64,
    }, // Fiat only
    Withdrawal {
        tx: TransactionBase,
        from: WalletId,
        amount: u64,
    }, // Fiat only
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBase {
    pub id: String,
    pub fee: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub is_taxable: bool,
    pub fee_price: Option<Decimal>, // For now in EUR, think about creating a Price<Currency> or Price<T>
}

impl Transaction {
    pub fn get_tx_base(&self) -> &TransactionBase {
        match self {
            Transaction::Transfer { tx, .. } => tx,
            Transaction::Trade { tx, .. } => tx,
            Transaction::Deposit { tx, .. } => tx,
            Transaction::Withdrawal { tx, .. } => tx,
        }
    }

    pub fn new_deposit(
        tx: TransactionBase,
        to: &Wallet,
        amount: u64,
    ) -> Result<Self, &'static str> {
        // Ensure the wallet type is Fiat
        if let Wallet::Fiat(_) = to {
            Ok(Transaction::Deposit {
                tx,
                to: to.get().id.clone(),
                amount,
            })
        } else {
            Err("Deposit transactions can only use Fiat wallets")
        }
    }

    pub fn new_withdrawal(
        tx: TransactionBase,
        from: &Wallet,
        amount: u64,
    ) -> Result<Self, &'static str> {
        // Ensure the wallet type is Fiat
        if let Wallet::Fiat(_) = from {
            Ok(Transaction::Withdrawal {
                tx,
                from: from.get().id.clone(),
                amount,
            })
        } else {
            Err("Withdrawal transactions can only use Fiat wallets")
        }
    }
}
