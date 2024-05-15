use chrono::Utc;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::env;

use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha384;

const API_BITFINEX_ENDPOINT: &str = "https://api.bitfinex.com";

#[derive(Deserialize, Serialize, Debug)]
pub struct BitfinexTrade {
    id: i32,
    symbol: String,
    mts: i32,
    order_id: i32,
    exec_amount: f64,
    exec_price: f64,
    order_type: Option<String>, // Use Option<String> to handle cases where ORDER_TYPE is not set
    order_price: f64,
    maker: i32,
    fee: f64,
    fee_currency: String,
    cid: i32,
}

#[derive(Debug, Deserialize)]
pub struct Trade {
    price: Vec<f64>,
    amount: Vec<f64>,
    exchange: Vec<String>,
    #[serde(rename = "type")]
    trade_type: Vec<String>, // "type" is a reserved keyword, so we rename it
    fee_currency: Vec<String>,
    fee_amount: Vec<f64>,
    tid: Vec<i32>,
    order_id: Vec<i32>,
}

#[tokio::main] // If the function is called often, it is preferable to create the runtime using the runtime builder so the runtime can be reused across calls.
pub async fn fetch_history_bitfinex(start: i64) -> Result<Vec<Trade>, Error> {
    let api_key = env::var("BITFINEX_KEY_ID").expect("BITFINEX_KEY_ID not set in .env file");
    let api_secret =
        env::var("BITFINEX_KEY_SECRET").expect("BITFINEX_KEY_SECRET not set in .env file");

    let nonce = Utc::now().timestamp_millis().to_string();
    // let endpoint = "auth/r/trades/hist";
    let endpoint = "mytrades";

    // let payload = json!({"start": start});
    // let message = format!("/api/v2/{}{}", endpoint, nonce) + &payload.to_string();

    //V1
    let url = "/v1/mytrades";
    let complete_url = format!("{}{}", API_BITFINEX_ENDPOINT, url);
    let body = serde_json::json!({
        "request": url,
        "nonce": nonce
    });

    let payload = base64::encode(&body.to_string());
    let signature = {
        let mut hmac =
            Hmac::<Sha384>::new_from_slice(api_secret.as_bytes()).expect("HMAC creation failed");
        hmac.update(payload.as_ref());
        hmac.finalize().into_bytes()
    };

    let sig = hex::encode(signature);
    let client = reqwest::Client::new();
    let response = client
        .post(&complete_url)
        .header("X-BFX-APIKEY", api_key)
        .header("X-BFX-PAYLOAD", payload)
        .header("X-BFX-SIGNATURE", sig)
        .json(&body)
        .send()
        .await?;

    return Ok(response.json::<Vec<Trade>>().await?);

    // let mut hmac = Hmac::<Sha384>::new_from_slice(api_secret.as_bytes()).expect("HMAC creation failed");
    // hmac.update(message.as_bytes());
    // let code_bytes = hmac.finalize().into_bytes();
    // let signature = hex::encode(code_bytes);

    // let mut file = File::create("data.txt").unwrap();
    // file.write_all(signature.as_bytes()).unwrap();

    // let mut headers = reqwest::header::HeaderMap::new();
    // headers.insert("bfx-apikey", reqwest::header::HeaderValue::from_str(&api_key).unwrap());
    // headers.insert("bfx-nonce", reqwest::header::HeaderValue::from_str(&nonce).unwrap());
    // headers.insert("bfx-signature", reqwest::header::HeaderValue::from_str(&hex::encode(signature)).unwrap());

    // let request_url = format!("{API_BITFINEX_ENDPOINT}/{endpoint}");
    // let client = reqwest::Client::new();
    // let response = client
    //                                     .post(request_url)
    //                                     .headers(headers)
    //                                     .json(&payload)
    //                                     .send()
    //                                     .await?;

    // let trades: Vec<BitfinexTrade> = response.json().await?;
    // return Ok(trades);
}
