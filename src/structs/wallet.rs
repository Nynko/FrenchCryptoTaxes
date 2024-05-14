use std::{collections::HashMap, ops::Add};

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
    // currency + address + platform should be unique
    pub id: String,
    pub currency: String,
    pub platform: Platform,
    pub address: Option<String>,
    pub owner: Owner,
    pub balance: Decimal,
    pub info: Option<String>,
}
