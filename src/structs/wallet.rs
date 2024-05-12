use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/* Data storage in several hash-maps for easy access of the id */
pub type Address = String;
pub type Currency = String;
pub type WalletId = String;
pub type AddrMap = HashMap<Address, WalletId>;
pub type PlatformMap = HashMap<Platform, AddrMap>;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletIdMap {
    ids: HashMap<Currency, PlatformMap>,
}

impl WalletIdMap {
    pub fn new() -> Self {
        return WalletIdMap {
            ids: HashMap::new(),
        };
    }

    pub fn get(&self, currency: String, platform: Platform, address: String) -> Option<WalletId> {
        return self
            .ids
            .get(&currency)
            .and_then(|map| map.get(&platform))
            .and_then(|map| map.get(&address))
            .cloned();
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
    pub balance: u64,
    pub info: Option<String>,
}
