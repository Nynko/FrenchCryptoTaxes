/* This is used to get price of a wallet depending on a Platform */

use rust_decimal::Decimal;

use crate::{
    api::kraken_mapping,
    errors::ApiError,
    structs::{Platform, Transaction, Wallet},
};

pub async fn get_price_api(tx: &Transaction, wallet: &Wallet) -> Result<Decimal, ApiError> {
    let time = tx.get_tx_base().timestamp;
    let currency = wallet.get().currency.clone();

    match wallet.get().platform {
        Platform::Kraken => {
            let price =
                kraken_mapping::get_currency_price(time.timestamp().to_string(), currency).await?;
            return Ok(price);
        }
        Platform::Binance => todo!(),
        Platform::Blockchain => todo!(),
        Platform::Other(_) => todo!(),
    }
}
