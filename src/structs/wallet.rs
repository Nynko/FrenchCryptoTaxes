use hashbrown::HashMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/* Data storage for easy access of the id */
pub type Address = Option<String>;
pub type Currency = String;
pub type WalletId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletIdMap {
    pub ids: HashMap<(Currency, Platform, Address), WalletId>,
}

impl WalletIdMap {
    pub fn new() -> Self {
        return WalletIdMap {
            ids: HashMap::new(),
        };
    }

    pub fn get(
        &self,
        currency: &String,
        platform: &Platform,
        address: &Option<String>,
    ) -> Option<WalletId> {
        return self
            .ids
            .get(&(currency.clone(), platform.clone(), address.clone()))
            .cloned();
    }

    pub fn insert(
        &mut self,
        currency: String,
        platform: Platform,
        address: Option<String>,
        wallet_id: String,
    ) -> Option<WalletId> {
        return self.ids.insert((currency, platform, address), wallet_id);
    }
}

/* A wallet correspond to a crypto Wallet. For French taxes, there is no distinction between wallets,
only the whole portfolio is considered.

A wallet can be a crypto wallet or a fiat wallet.
*/
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Wallet {
    Fiat(WalletBase),
    Crypto(WalletBase),
}

impl Wallet {
    pub fn is_crypto(&self) -> bool {
        match self {
            Wallet::Fiat(_) => false,
            Wallet::Crypto(_) => true,
        }
    }

    pub fn get(&self) -> &WalletBase {
        match self {
            Wallet::Fiat(base) => base,
            Wallet::Crypto(base) => base,
        }
    }

    pub fn get_id(&self) -> String {
        match self {
            Wallet::Fiat(base) => base.id.clone(),
            Wallet::Crypto(base) => base.id.clone(),
        }
    }

    pub fn get_currency(&self) -> Currency {
        match self {
            Wallet::Fiat(base) => base.currency.clone(),
            Wallet::Crypto(base) => base.currency.clone(),
        }
    }

    pub fn get_mut(&mut self) -> &mut WalletBase {
        match self {
            Wallet::Fiat(base) => base,
            Wallet::Crypto(base) => base,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Owner {
    User,
    Platform,
    Other,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Binance,
    Kraken,
    Blockchain,
    Other(String),
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct WalletBase {
    // currency + address + platform should be unique ---> Maybe we should implement Eq, PartialEq, and Hash traits as so, so we don't need to use WalletIdMap
    pub id: WalletId,
    pub currency: String,
    pub platform: Platform,
    pub address: Option<String>,
    pub owner: Owner,
    pub balance: Decimal,
    pub info: Option<String>,
}

/* Correspond to a snapshot of the wallet state (balance and potentially price) before a transaction. (It is always associated to a transaction)
The fee is included here as it is taken from the wallet state. The fee is an information relative to the Wallet State: the balance and also to the cost_basis.

We don't considere the fee to be a taxable event has it would greatly complexify the transactions with many micro-transaction and imply that CryptoToCrypto
transfer with fee would make a taxable event.
This is a choice, and tax over fees are possible to be implemented.
*/
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct _WalletSnapshot<Price>
where
    Price: Eq + PartialEq,
{
    pub id: WalletId,
    pub pre_tx_balance: Decimal,
    pub price_eur: Price, 
    pub fee: Option<Decimal>,
}


// About Price: 
// For transaction: we need the price associated with the amount for taxable trade/transfer or if there is a fee. To ensure immutability of the data, it is preferable to just get the price everytime + it is something usually useful for user experience.
// For Portfolio: we want the price to be optional to only get the price when we want
pub type WalletSnapshot = _WalletSnapshot<Decimal>;
pub type PortfolioWalletSnapshot = _WalletSnapshot<Option<Decimal>>;

impl WalletSnapshot {
    pub fn to_portfolio(&self) -> PortfolioWalletSnapshot{
        return PortfolioWalletSnapshot{
            id: self.id.clone(),
            pre_tx_balance: self.pre_tx_balance,
            price_eur: Some(self.price_eur),
            fee: self.fee,
        }
    }
}
