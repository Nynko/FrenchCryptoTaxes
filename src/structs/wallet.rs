use std::collections::HashMap;

use super::Currency;

/* Data storage in several hash-maps for easy access */
pub type AddrMap<'a> = HashMap<&'a str, Wallet<'a>>;
pub type PlatformMap<'a> = HashMap<&'a str, AddrMap<'a>>;
pub type WalletMap<'a> = HashMap<&'a str, PlatformMap<'a>>;

/* A wallet correspond to a crypto Wallet. For French taxes, there is no distinction between wallets,
only the whole portfolio is considered.

A wallet can be a crypto wallet or a fiat wallet.
*/
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Wallet<'a> {
    Fiat(WalletBase<'a>),
    Crypto(WalletBase<'a>),
}

impl<'a> Wallet<'a> {
    pub fn get(&self) -> &WalletBase {
        match self {
            Wallet::Fiat(base) => base,
            Wallet::Crypto(base) => base,
        }
    }

    pub fn get_mut(&mut self) -> &'a mut WalletBase {
        match self {
            Wallet::Fiat(base) => base,
            Wallet::Crypto(base) => base,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Owner {
    User,
    Platform,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum Platform {
    Binance,
    Kraken,
    Blockchain,
    Other(String),
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct WalletBase<'a> {
    // currency + address + platform should be unique
    pub currency: &'a Currency,
    pub platform: Platform,
    pub address: Option<String>,
    pub owner: Owner,
    pub balance: u64,
    pub info: Option<String>,
}
