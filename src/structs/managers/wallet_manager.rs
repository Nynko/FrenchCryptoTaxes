use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::structs::{Wallet, WalletIdMap};

use super::Persistable;

/* This wallet manager will handle saving the data and loading the previous data if they exist.
It will implement de Drop trait to save.*/
#[derive(Serialize, Deserialize)]
pub struct WalletManager {
    pub wallets: HashMap<String, Wallet>,
    pub wallet_ids: WalletIdMap,
    path: String,
    persist: bool,
}

impl Persistable for WalletManager {
    const PATH: &'static str = ".data/wallets";

    fn default_new(path: String, persist: bool) -> Self {
        Self {
            wallets: HashMap::new(),
            wallet_ids: WalletIdMap {
                ids: HashMap::new(),
            },
            path,
            persist
        }
    }

    fn get_path(&self) -> &str{
        return &self.path;
    }

    fn is_persistent(&self) -> bool{
        return self.persist;
    }
}

impl Drop for WalletManager {
    fn drop(&mut self) {
        if self.persist{
            let _save = self.save();
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{thread::sleep, time::Duration};

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
        sleep(Duration::from_millis(10));
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
