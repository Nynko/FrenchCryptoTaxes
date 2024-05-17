use std::fs::File;

use hashbrown::HashMap;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

use crate::{
    errors::IoError,
    structs::{Wallet, WalletIdMap},
    utils::{create_directories_if_needed, file_exists},
};

/* This wallet manager will handle saving the data and loading the previous data if they exist.
It will implement de Drop trait to save.*/
#[derive(Serialize, Deserialize)]
pub struct WalletManager {
    pub wallets: HashMap<String, Wallet>,
    pub wallet_ids: WalletIdMap,
    path: String,
}

impl WalletManager {
    pub const PATH: &'static str = ".data/wallets";

    pub fn new(path: Option<String>) -> Result<Self, IoError> {
        // Load wallets here or create empty Vec
        let path = if path.is_some() {
            path.unwrap()
        } else {
            Self::PATH.to_string()
        };
        if !file_exists(&path) {
            return Ok(Self {
                wallets: HashMap::new(),
                wallet_ids: WalletIdMap {
                    ids: HashMap::new(),
                },
                path,
            });
        } else {
            let file = File::open(path).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized_map: WalletManager =
                rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
            return Ok(deserialized_map);
        }
    }

    pub fn save(&self) -> Result<(), IoError> {
        create_directories_if_needed(&self.path);
        let file = File::create(&self.path).map_err(|e| IoError::new(e.to_string()))?;
        let mut writer = Serializer::new(file);
        self.serialize(&mut writer)
            .map_err(|e| IoError::new(e.to_string()))?;
        return Ok(());
    }
}

impl Drop for WalletManager {
    fn drop(&mut self) {
        let _save = self.save();
    }
}

#[cfg(test)]
mod tests {

    use serial_test::serial;

    use crate::structs::wallet::Platform;

    use super::*;

    #[test]
    fn test_save() {
        let mut wallet_manager =
            WalletManager::new(Some(".data_test/wallet".to_string())).unwrap();

        wallet_manager.wallet_ids.ids.insert(
            ("test".to_string(), Platform::Binance, None),
            "test".to_string(),
        );

        wallet_manager.save().unwrap();

        let wallet_manager2 = WalletManager::new(Some(".data_test/wallet".to_string())).unwrap();

        assert_eq!(
            wallet_manager2
                .wallet_ids
                .get(&"test".to_string(), &Platform::Binance, &None),
            Some("test".to_string())
        );
    }

    #[test]
    fn test_drop() {
        {
            let mut wallet_manager =
                WalletManager::new(Some(".data_test/wallet_drop".to_string())).unwrap();

            wallet_manager.wallet_ids.ids.insert(
                ("test2".to_string(), Platform::Binance, None),
                "test2".to_string(),
            );
        }

        let wallet_manager =
            WalletManager::new(Some(".data_test/wallet_drop".to_string())).unwrap();

        assert_eq!(
            wallet_manager
                .wallet_ids
                .get(&"test2".to_string(), &Platform::Binance, &None),
            Some("test2".to_string())
        );
    }

    #[test]
    #[should_panic]
    #[serial]
    fn test_drop_after_panic() {
        let mut wallet_manager =
            WalletManager::new(Some(".data_test/wallet_drop_panic".to_string())).unwrap();
        wallet_manager.wallet_ids.ids.insert(
            ("test3".to_string(), Platform::Binance, None),
            "test3".to_string(),
        );
        panic!("test")
    }

    #[test]
    #[serial]
    fn test_drop_after_panic_part2() {
        let wallet_manager =
            WalletManager::new(Some(".data_test/wallet_drop_panic".to_string())).unwrap();
        assert_eq!(
            wallet_manager
                .wallet_ids
                .get(&"test3".to_string(), &Platform::Binance, &None),
            Some("test3".to_string())
        );
    }
}
