use super::{GlobalCostBasis, Wallet, WalletSnapshot};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
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
/* Transaction are meant to be immutable, additionnal data should be held separated in HashMaps<TransactionId,..> for instance */
#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    // Transfer can be a "local" non taxable transfer, it can be a taxable transfer to external entity
    Transfer {
        tx: TransactionBase,
        from: WalletSnapshot,
        to: WalletSnapshot,
        amount: Decimal,
        income : Option<Income>, // Income correspond to a Crypto Transfer to from and to the same wallet that can be a reward, a staking interest, an airdrop, a mining, or a payment in crypto 
        cost_basis: GlobalCostBasis,
    },
    // Trade can be a Crypto/Crypto non taxable trade, or taxable sold of Crypto, or non taxable event: buying crypto
    Trade {
        tx: TransactionBase, 
        from: WalletSnapshot,
        to: WalletSnapshot,
        exchange_pair: Option<(String, String)>,
        sold_amount: Decimal,
        bought_amount: Decimal,
        trade_type: TradeType,
        cost_basis: GlobalCostBasis,
    },
    Deposit {
        tx: TransactionBase,
        to: WalletSnapshot,
        amount: Decimal,
    }, // Fiat only
    Withdrawal {
        tx: TransactionBase,
        from: WalletSnapshot,
        amount: Decimal,
    }, // Fiat only
}

/* The Trade type:
If FiatToCrypto : Representation of the transaction cost basis.
This is used to calculate the global cost basis when iteratively treating the data.
*/
#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TradeType {
    FiatToCrypto { local_cost_basis: Decimal },
    CryptoToFiat,
    CryptoToCrypto,
}


#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Income {
    value: Decimal,
    subtype : IncomeType
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum IncomeType{
    Airdrop,
    Hardfork,
    Income,
    Interest,
    Mining,
    Staking,
    Gift,
    Donation,
    Other(String)
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBase {
    pub id: String, // This should not be generated but come from an external source  OR if not possible deterministically created from "uniqueness" element of the transaction (timestamp, fee, wallet_ids...)
    pub timestamp: DateTime<Utc>,
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

    pub fn is_trade_or_transfer(&self) -> bool {
        match self {
            Transaction::Trade { .. } => true,
            Transaction::Transfer { .. } => true,
            Transaction::Deposit { .. } => false,
            Transaction::Withdrawal { .. } => false,
        }
    }
    /* To delete after moving the cost basis outside of transaction enum */
    pub fn tmp_get_cost_basis(&self) -> Option<&GlobalCostBasis> {
        match self {
            Transaction::Trade { cost_basis, .. } => Some(cost_basis),
            Transaction::Transfer { cost_basis, .. } => Some(cost_basis),
            Transaction::Deposit { .. } => None,
            Transaction::Withdrawal { .. } => None,
        }
    }

    /* Determine if a transaction is taxable (outside of if it has been marked taxable by the user in the portfolio).
    There is only one case were we know a transaction is for sure taxable: Trading to fiat */
    pub fn is_taxable(&self) -> bool {
        match self {
            Transaction::Trade { trade_type, .. } => match trade_type {
                TradeType::CryptoToFiat => true,
                _ => false
            } ,
            _ => false
        }
    }

    pub fn new_deposit(
        tx: TransactionBase,
        to: &Wallet,
        amount: Decimal,
        fee: Option<Decimal>,
        price_eur: Decimal,
    ) -> Result<Self, &'static str> {
        // Ensure the wallet type is Fiat
        if let Wallet::Fiat(_) = to {
            Ok(Transaction::Deposit {
                tx,
                to: WalletSnapshot {
                    id: to.get().id.clone(),
                    pre_tx_balance: dec!(0),
                    fee,
                    price_eur,
                },
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
        fee: Option<Decimal>,
        price_eur: Decimal,
    ) -> Result<Self, &'static str> {
        // Ensure the wallet type is Fiat
        if let Wallet::Fiat(_) = from {
            Ok(Transaction::Withdrawal {
                tx,
                from: WalletSnapshot {
                    id: from.get().id.clone(),
                    pre_tx_balance: dec!(0),
                    fee,
                    price_eur,
                },
                amount,
            })
        } else {
            Err("Withdrawal transactions can only use Fiat wallets")
        }
    }
}
