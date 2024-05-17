use base64::prelude::*;
use chrono::Utc;
use hashbrown::{HashMap, HashSet};
use hmac::{Hmac, Mac};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Error,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::{env, time::Duration};
use tokio::time::sleep;

use crate::errors::ApiError;
const API_KRAKEN_ENDPOINT: &str = "https://api.kraken.com";

/* As per documentation signature is HMAC-SHA512 of (URI path + SHA256(nonce + POST data)) and base64 decoded secret API key.
https://docs.kraken.com/api/docs/guides/spot-rest-auth
*/
fn get_kraken_signature(
    urlpath: &String,
    url_encoded_data: &String,
    secret: &Vec<u8>,
    nonce: &String,
) -> String {
    let postdata = format!("{}{}", nonce, url_encoded_data);
    let message = [
        urlpath.as_bytes(),
        Sha256::digest(postdata.as_bytes()).as_slice(),
    ]
    .concat();
    let mut mac = Hmac::<Sha512>::new_from_slice(secret).expect("Wrong Key size");
    mac.update(&message);
    let result = mac.finalize().into_bytes().to_vec();
    return BASE64_STANDARD.encode(&result);
}

async fn fetch_data<T: for<'a> Deserialize<'a>>(
    url_path: &String,
    params: &mut HashMap<&str, String>,
    api_key: &String,
    api_secret: &String,
) -> Result<Response<T>, Error> {
    let nonce = (Utc::now().timestamp_millis() * 1000).to_string();
    params.insert("nonce", nonce.clone());
    let url = format!("{API_KRAKEN_ENDPOINT}{url_path}");

    let encoded = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.clone())
        .finish();

    let decoded_secret = BASE64_STANDARD.decode(api_secret).unwrap();

    let signature = get_kraken_signature(&url_path, &encoded, &decoded_secret, &nonce);

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_str("application/x-www-form-urlencoded").unwrap(),
    );
    headers.insert(
        "Accept",
        HeaderValue::from_str("application/x-www-form-urlencoded").unwrap(),
    );
    headers.insert("API-Key", HeaderValue::from_str(api_key).unwrap());
    headers.insert("API-Sign", HeaderValue::from_str(&signature).unwrap());

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .headers(headers)
        .form(params)
        .send()
        .await
        .unwrap();

    let text = response.text().await.unwrap();

    let trade_response: Response<T> = serde_json::from_str(&text).unwrap();

    return Ok(trade_response);
}

async fn fetch_ledger_data(
    api_key: &String,
    api_secret: &String,
    counter: &mut u8,
    tier: &Tier,
) -> Result<Vec<LedgerHistory>, ApiError> {
    let mut ledger_history: Vec<LedgerHistory> = Vec::new();
    let mut unique_entries: HashSet<String> = HashSet::new();
    let url_path = String::from("/0/private/Ledgers");
    let mut params = HashMap::new();

    let ledger_response: Response<LedgersInfo> =
        fetch_data(&url_path, &mut params, &api_key, &api_secret)
            .await
            .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
    *counter += 2;
    if !ledger_response.error.is_empty() {
        return Err(ApiError::ApiCallError(ledger_response.error.concat()));
    }
    let mut count: u32 = 0;
    if let Some(result) = ledger_response.result {
        count = result.count;
        for (key, entry) in result.ledger.iter() {
            if unique_entries.insert(key.to_string()) {
                ledger_history.push(entry.clone());
            }
        }
    }
    for offset in (50..count).step_by(50) {
        params.insert("ofs", offset.to_string());
        if tier.is_above_limit(*counter, 2) {
            sleep(tier.get_sleep_time(2)).await;
            *counter -= 2;
        }
        let response: Response<LedgersInfo> =
            fetch_data(&url_path, &mut params, &api_key, &api_secret)
                .await
                .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
        *counter += 2;
        if !response.error.is_empty() {
            return Err(ApiError::ApiCallError(response.error.concat()));
        }
        if let Some(result) = response.result {
            for (key, entry) in result.ledger.iter() {
                if unique_entries.insert(key.to_string()) {
                    ledger_history.push(entry.clone());
                }
            }
        }
    }

    ledger_history.sort_by(|a, b| a.time.total_cmp(&b.time));

    return Ok(ledger_history);
}

async fn fetch_trade_history(
    api_key: &String,
    api_secret: &String,
    counter: &mut u8,
    tier: &Tier,
) -> Result<HashMap<String, TradeInfo>, ApiError> {
    let url_path = String::from("/0/private/TradesHistory");
    let mut params = HashMap::new();
    let mut trade_history = HashMap::new();
    let trade_response: Response<TradeHistory> =
        fetch_data(&url_path, &mut params, &api_key, &api_secret)
            .await
            .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
    *counter += 2;
    if !trade_response.error.is_empty() {
        return Err(ApiError::ApiCallError(trade_response.error.concat()));
    }
    let mut count: u32 = 0;
    if let Some(result) = trade_response.result {
        count = result.count;
        trade_history.extend(result.trades);
    }
    for offset in (50..count).step_by(50) {
        params.insert("ofs", offset.to_string());
        if tier.is_above_limit(*counter, 2) {
            sleep(tier.get_sleep_time(2)).await;
            *counter -= 2;
        }
        let response: Response<TradeHistory> =
            fetch_data(&url_path, &mut params, &api_key, &api_secret)
                .await
                .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
        *counter += 2;
        if !response.error.is_empty() {
            return Err(ApiError::ApiCallError(response.error.concat()));
        }
        if let Some(result) = response.result {
            trade_history.extend(result.trades);
        }
    }

    return Ok(trade_history);
}

async fn fetch_deposit_data(
    api_key: &String,
    api_secret: &String,
    counter: &mut u8,
    tier: &Tier,
) -> Result<HashMap<String, Deposit>, ApiError> {
    let mut deposit_history: HashMap<String, Deposit> = HashMap::new();
    let url_path = String::from("/0/private/DepositStatus");
    let mut params = HashMap::new();
    let mut next_cursor: Option<String> = Some(String::from("true"));

    while next_cursor.is_some() {
        params.insert("cursor", next_cursor.unwrap());
        if tier.is_above_limit(*counter, 1) {
            sleep(tier.get_sleep_time(1)).await;
            *counter -= 1;
        }
        let deposit_response: Response<DepositInfo> =
            fetch_data(&url_path, &mut params, &api_key, &api_secret)
                .await
                .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
        *counter += 1;

        if !deposit_response.error.is_empty() {
            return Err(ApiError::ApiCallError(deposit_response.error.concat()));
        }

        if let Some(result) = deposit_response.result {
            if let Some(deposits) = result.deposits {
                for deposit in deposits {
                    deposit_history.insert(deposit.refid.clone(), deposit);
                }
            }
            next_cursor = result.next_cursor;
        } else {
            next_cursor = None;
        }
    }

    return Ok(deposit_history);
}

async fn fetch_withdraw_data(
    api_key: &String,
    api_secret: &String,
    counter: &mut u8,
    tier: &Tier,
) -> Result<HashMap<String, Withdrawal>, ApiError> {
    let mut withdraw_history: HashMap<String, Withdrawal> = HashMap::new();
    let url_path = String::from("/0/private/WithdrawStatus");
    let mut params = HashMap::new();
    let mut next_cursor: Option<String> = Some(String::from("true"));

    while next_cursor.is_some() {
        params.insert("cursor", next_cursor.unwrap());
        if tier.is_above_limit(*counter, 1) {
            sleep(tier.get_sleep_time(1)).await;
            *counter -= 1;
        }
        let deposit_response: Response<WithdrawalInfo> =
            fetch_data(&url_path, &mut params, &api_key, &api_secret)
                .await
                .map_err(|e| ApiError::ApiCallError(e.to_string()))?;
        *counter += 1;

        if !deposit_response.error.is_empty() {
            return Err(ApiError::ApiCallError(deposit_response.error.concat()));
        }

        if let Some(result) = deposit_response.result {
            if let Some(withdraws) = result.withdrawals {
                for withdraw in withdraws {
                    withdraw_history.insert(withdraw.refid.clone(), withdraw);
                }
            }
            next_cursor = result.next_cursor;
        } else {
            next_cursor = None;
        }
    }

    return Ok(withdraw_history);
}

pub async fn fetch_specific_trade_data(
    time: String,
    trading_pair: String,
) -> Result<Response<TickData>, ApiError> {
    let url = "/0/public/Trades";
    let mut params = HashMap::new();
    params.insert("pair", trading_pair);
    params.insert("since", time);
    params.insert("count", 6.to_string());
    let encoded = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.clone())
        .finish();

    let full_url = [API_KRAKEN_ENDPOINT, url, "?", &encoded].concat();

    let headers = HeaderMap::new();

    let client = reqwest::Client::new();
    let response = client.get(full_url).headers(headers).send().await.unwrap();

    let text = response.text().await.unwrap();

    let trade_response: Response<TickData> =
        serde_json::from_str(&text).map_err(|e| ApiError::DeserializationError(e.to_string()))?;

    if !trade_response.error.is_empty() {
        return Err(ApiError::ApiCallError(trade_response.error.concat()));
    }

    return Ok(trade_response);
}

#[tokio::main]
pub async fn fetch_assets_pair() -> Result<Response<AssetPairs>, ApiError> {
    let url = "/0/public/AssetPairs";
    let full_url = [API_KRAKEN_ENDPOINT, url].concat();
    let headers = HeaderMap::new();
    let client = reqwest::Client::new();
    let response = client.get(full_url).headers(headers).send().await.unwrap();

    let text = response.text().await.unwrap();

    let trade_response: Response<AssetPairs> = serde_json::from_str(&text).unwrap();

    if !trade_response.error.is_empty() {
        return Err(ApiError::DeserializationError(
            trade_response.error.concat(),
        ));
    }

    return Ok(trade_response);
}

pub type HistoryResponse = (
    Vec<LedgerHistory>,
    HashMap<String, TradeInfo>,
    HashMap<String, Deposit>,
    HashMap<String, Withdrawal>,
);

#[tokio::main]
pub async fn fetch_history_kraken(tier: Tier) -> Result<HistoryResponse, ApiError> {
    let api_key = env::var("KRAKEN_KEY").expect("KRAKEN_KEY not set in .env file");
    let api_secret: String =
        env::var("KRAKEN_SECRET").expect("KRAKEN_SECRET not set in .env file");
    let mut api_counter: u8 = 0; // Counter limit depending on Tier:  see https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits

    let ledger_history: Vec<LedgerHistory> =
        fetch_ledger_data(&api_key, &api_secret, &mut api_counter, &tier).await?;
    let trade_history: HashMap<String, TradeInfo> =
        fetch_trade_history(&api_key, &api_secret, &mut api_counter, &tier).await?;
    let deposits: HashMap<String, Deposit> =
        fetch_deposit_data(&api_key, &api_secret, &mut api_counter, &tier).await?;
    let withdrawals: HashMap<String, Withdrawal> =
        fetch_withdraw_data(&api_key, &api_secret, &mut api_counter, &tier).await?;

    return Ok((ledger_history, trade_history, deposits, withdrawals));
}

pub enum Tier {
    Starter,
    Intermediate,
    Pro,
}

impl Tier {
    // Define associated constants for each variant
    pub const STARTER_DECAY_RATE: f32 = 0.33;
    pub const STARTER_MAX_COUNTER: u8 = 15;

    pub const INTERMEDIATE_DECAY_RATE: f32 = 0.5;
    pub const INTERMEDIATE_MAX_COUNTER: u8 = 20;

    pub const PRO_DECAY_RATE: f32 = 1.0;
    pub const PRO_MAX_COUNTER: u8 = 20;

    pub fn is_above_limit(&self, counter: u8, step: u8) -> bool {
        match self {
            Tier::Starter => counter + step > Self::STARTER_MAX_COUNTER,
            Tier::Intermediate => counter + step > Self::INTERMEDIATE_MAX_COUNTER,
            Tier::Pro => counter + step > Self::PRO_MAX_COUNTER,
        }
    }

    pub fn get_decay_rate(&self) -> f32 {
        match self {
            Tier::Starter => Self::STARTER_DECAY_RATE,
            Tier::Intermediate => Self::INTERMEDIATE_DECAY_RATE,
            Tier::Pro => Self::PRO_DECAY_RATE,
        }
    }

    pub fn get_sleep_time(&self, step: u8) -> Duration {
        match self {
            Tier::Starter => Duration::from_secs_f32(step as f32 / Self::STARTER_DECAY_RATE),
            Tier::Intermediate => {
                Duration::from_secs_f32(step as f32 / Self::INTERMEDIATE_DECAY_RATE)
            }
            Tier::Pro => Duration::from_secs_f32(step as f32 / Self::PRO_DECAY_RATE),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TradeEntry {
    pub price: String,
    pub volume: String,
    pub time: f64,
    pub buy_sell: String,
    pub market_limit: String,
    pub miscellaneous: String,
    pub trade_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TickData {
    #[serde(flatten)]
    pub trades: HashMap<String, Vec<TradeEntry>>,
    pub last: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct AssetPair {
    pub altname: String,
    pub wsname: String,
    pub base: String,
    pub quote: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssetPairs {
    #[serde(flatten)]
    pub pairs: HashMap<String, AssetPair>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Deposit {
    pub method: String,
    pub network: Option<String>,
    pub aclass: String,
    pub asset: String,
    pub refid: String,
    pub txid: String,
    pub info: Option<String>,
    pub amount: Decimal,
    pub fee: Decimal,
    pub time: i32,
    pub status: String,
    #[serde(rename = "status-prop")]
    pub status_prop: Option<String>,
    pub originators: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Withdrawal {
    pub method: String,
    pub network: Option<String>,
    pub aclass: String,
    pub asset: String,
    pub refid: String,
    pub txid: String,
    pub info: Option<String>,
    pub amount: Decimal,
    pub fee: Decimal,
    pub time: i32,
    pub status: String,
    #[serde(rename = "status-prop")]
    pub status_prop: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DepositInfo {
    pub deposits: Option<Vec<Deposit>>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct WithdrawalInfo {
    pub withdrawals: Option<Vec<Withdrawal>>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Response<T> {
    pub error: Vec<String>,
    pub result: Option<T>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    None,
    Trade,
    Deposit,
    Withdrawal,
    Transfer,
    Margin,
    Adjustment,
    Rollover,
    Spend,
    Receive,
    Settled,
    Credit,
    Staking,
    Reward,
    Dividend,
    Sale,
    Conversion,
    NftTrade,
    NftCreatorFee,
    NftRebate,
    CustodyTransfer,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LedgerHistory {
    pub refid: String,
    pub time: f64,
    pub r#type: EntryType,
    pub subtype: String,
    pub aclass: String,
    pub asset: String,
    pub amount: Decimal,
    pub fee: Decimal,
    pub balance: Decimal,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LedgersInfo {
    pub ledger: HashMap<String, LedgerHistory>,
    pub count: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TradeHistory {
    pub trades: HashMap<String, TradeInfo>,
    pub count: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TradeInfo {
    pub ordertxid: String,
    pub postxid: String,
    pub pair: String,
    pub time: f64,
    pub r#type: String,
    pub ordertype: String,
    pub price: String,
    pub cost: String,
    pub fee: Decimal,
    pub vol: String,
    pub margin: String,
    pub misc: String,
    pub trade_id: i64,
    pub maker: bool,
    pub posstatus: Option<String>,
    pub cprice: Option<f64>,
    pub ccost: Option<f64>,
    pub cfee: Option<f64>,
    pub cvol: Option<f64>,
    pub cmargin: Option<f64>,
    pub net: Option<f64>,
    pub trades: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_signature() {
        /* https://docs.kraken.com/api/docs/guides/spot-rest-auth */
        let secret = String::from("kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==");
        let nonce = String::from("1616492376594");
        let payload = String::from(
            "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25",
        );
        let uri_path = String::from("/0/private/AddOrder");
        let decoded_secret = BASE64_STANDARD.decode(secret).unwrap();

        let signature = get_kraken_signature(&uri_path, &payload, &decoded_secret, &nonce);

        assert_eq!(signature,String::from("4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="));
    }

    #[test]
    fn test_deserialize_ledger() {
        let json_data = r#"
         {
            "error": [],
            "result": {
              "ledger": {
                "L4UESK-KG3EQ-UFO4T5": {
                  "refid": "TJKLXF-PGMUI-4NTLXU",
                  "time": 1688464484.1787,
                  "type": "trade",
                  "subtype": "",
                  "aclass": "currency",
                  "asset": "ZGBP",
                  "amount": "-24.5000",
                  "fee": "0.0490",
                  "balance": "459567.9171"
                },
                "LMKZCZ-Z3GVL-CXKK4H": {
                  "refid": "TBZIP2-F6QOU-TMB6FY",
                  "time": 1688444262.8888,
                  "type": "trade",
                  "subtype": "",
                  "aclass": "currency",
                  "asset": "ZUSD",
                  "amount": "0.9852",
                  "fee": "0.0010",
                  "balance": "52732.1132"
                }
              },
              "count": 2
            }
          }
    "#;

        let _trade_response: Response<LedgersInfo> = serde_json::from_str(json_data).unwrap();
    }

    #[test]
    fn test_deserialize_withdrawals() {
        let json_str = r#"{
            "error": [],
            "result": {
                "withdrawals": [
                    {
                        "method": "Tether USD (SPL)",
                        "network": "Solana",
                        "aclass": "currency",
                        "asset": "USDT",
                        "refid": "FTPLOMm-Ts8nbGkIZhCojd5y4JQybq",
                        "txid": "2kjytCtqJfEamCUK8qVQyVkUSEWp4vQe6NoSLB2DNtA3WPoAAy6dJkuQZD9vv686M9mvFpPXRMkG1nDfQ91ZRizR",
                        "info": "7uh6DW8Yv6nwjBtZccwUnq2Fx5Rn1f56cmuh1fKfYubK",
                        "amount": "1073.00000000",
                        "fee": "1.00000000",
                        "time": 1712856445,
                        "status": "Success",
                        "key": "Kucoin Sol"
                    }
                ],
                "next_cursor": null
            }
        }"#;

        // Deserialize the JSON string into a Response struct
        let _response: Response<WithdrawalInfo> = serde_json::from_str(json_str).unwrap();
        println!("{:?}", _response);
    }

    #[test]
    fn test_deserialize_deposits() {
        let json_str = r#"{
            "error": [],
            "result": {
                "deposits": [
                    {
                        "method": "USDT - Optimism (Unified)",
                        "aclass": "currency",
                        "asset": "USDT",
                        "refid": "FTPJYfm-g1G1Lf1ENRKbWykn1NuclP",
                        "txid": "0x2a793d4a88e6e8dc638d90d7ec53c3f91989c7ab8cdc1f129a98dbefe2b6b31a",
                        "info": "0x047fd491864c01d37305e26ff3a62af27cd1269f",
                        "amount": "156.03956000",
                        "fee": "0.00000000",
                        "time": 1713102631,
                        "status": "Success"
                    }
                ],
                "next_cursor": null
            }
        }"#;

        // Deserialize the JSON string into a Response struct
        let _response: Response<DepositInfo> = serde_json::from_str(json_str).unwrap();

        println!("{:?}", _response);
    }
}
