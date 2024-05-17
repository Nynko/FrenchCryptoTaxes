use std::fmt;

use crate::structs::{TransactionId, WalletId};

use super::ApiError;

#[derive(Debug, Clone)]
pub enum PortfolioHistoryError {
    MissingPreviousStateWallet {
        wallet_id: WalletId,
        tx_id: TransactionId,
    },
    FailureGettingPrice(ApiError),
}

impl fmt::Display for PortfolioHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PortfolioHistoryError::MissingPreviousStateWallet { wallet_id, tx_id } => {
                write!(f,"Wallet {} was not present in previous state when treating Tx {} or with a zero balance", wallet_id,tx_id)
            }
            PortfolioHistoryError::FailureGettingPrice(api_error) => api_error.fmt(f),
        }
    }
}
