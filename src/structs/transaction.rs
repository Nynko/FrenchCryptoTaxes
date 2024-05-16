use super::{GlobalCostBasis, Wallet, WalletId};
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
    - it can be a self-transaction. This can happen in the blockchain, although it won't often happen in exchanges (often exchanges use another currency for stacking...
                                    it ends up creating a new wallet for us. Although we don't often need to take the wallets associated with staking and just ignored related txs.
                                    Even if in the future we do: we should be able to easily hide these are they are making it more difficult to read the important txs)


Deposit: should never be from the same wallet, there is always an "external" wallet
         we represent that by a wallet that you own (like your bank acc) or that you don't own
         if it comes from the same wallet, it means it is an "income" (either a gift, reward...): money creation from nowhere basically

         Deposits are only available from Fiat Wallet
*/

pub type TransactionId = String;
/* We only have two types of transactions here:

A simple fee would be a transfer transaction to a wallet not owned by the user*/
#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    // Transfer can be a "local" non taxable transfer, it can be a taxable transfer to external entity
    Transfer {
        tx: TransactionBase,
        from: WalletId,
        to: WalletId,
        amount: Decimal,
        taxable: Option<Taxable>,
        cost_basis: GlobalCostBasis,
    },
    // Trade can be a Crypto/Crypto non taxable trade, or taxable sold of Crypto, or non taxable event: buying crypto
    Trade {
        tx: TransactionBase,
        from: WalletId,
        to: WalletId,
        exchange_pair: Option<(String, String)>,
        sold_amount: Decimal,
        bought_amount: Decimal,
        trade_type: TradeType,
        taxable: Option<Taxable>,
        cost_basis: GlobalCostBasis,
    },
    Deposit {
        tx: TransactionBase,
        to: WalletId,
        amount: Decimal,
    }, // Fiat only
    Withdrawal {
        tx: TransactionBase,
        from: WalletId,
        amount: Decimal,
    }, // Fiat only
}

/* The Trade type:
If FiatToCrypto :  Representation of the transaction cost basis.
This is used to calculate the global cost basis when iteratively treating the data.
*/
#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TradeType {
    FiatToCrypto { local_cost_basis: Decimal },
    CryptoToFiat,
    CryptoToCrypto,
}

/*The pf_total_value should be set depending on the global value of the portfolio before each transaction (at least each taxable one).
It can be caculated from the price of all wallets at an instant t.
The issue is getting price at instant t may take time (calling API). We want to get that information before actually treating the transaction,
when we only want it for taxable events.*/
#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Taxable {
    // Currently in EUR
    pub is_taxable: bool,
    pub price_eur: Decimal,
    pub pf_total_value: Decimal,      // Portfolio total value in euro
    pub is_pf_total_calculated: bool, // Each time recalculation is needed, this should be set to false (Recalculation use the PortfolioManager)
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBase {
    pub id: String, // This should not be generated but come from an external source  OR if not possible deterministically created from "uniqueness" element of the transaction (timestamp, fee, wallet_ids...)
    pub fee: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
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
        amount: Decimal,
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
        amount: Decimal,
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
