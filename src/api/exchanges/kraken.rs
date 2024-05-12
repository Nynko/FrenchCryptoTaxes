use std::{collections::HashMap, env};

use base64::prelude::*;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use serde::{Deserialize, Serialize};
use reqwest::{header::{HeaderMap, HeaderValue}, Error};
const API_KRAKEN_ENDPOINT: &str = "https://api.kraken.com";

/* As per documentation signature is HMAC-SHA512 of (URI path + SHA256(nonce + POST data)) and base64 decoded secret API key.
https://docs.kraken.com/api/docs/guides/spot-rest-auth
*/
fn get_kraken_signature(urlpath: &String, url_encoded_data: &String, secret: &Vec<u8>, nonce: &String) -> String {
    let postdata = format!("{}{}", nonce, url_encoded_data);
    let message = [urlpath.as_bytes(), Sha256::digest(postdata.as_bytes()).as_slice()].concat();
    let mut mac = Hmac::<Sha512>::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(&message);
    let result = mac.finalize().into_bytes().to_vec();
    return BASE64_STANDARD.encode(&result);
}

async fn fetch_data(api_key: &String, api_secret: &String, start_date: Option<i64>) -> Result<TradeResponse,Error>{
    let nonce = (Utc::now().timestamp_millis()*1000).to_string();
    let url_path = String::from("/0/private/TradesHistory");
    let url = format!("{API_KRAKEN_ENDPOINT}{url_path}");


    let mut params = HashMap::new();
    params.insert("nonce", nonce.clone());
    params.insert("type", String::from("all"));
    params.insert("trades", String::from("False"));
    params.insert("consolidate_taker", String::from("True"));
    if start_date.is_some(){
        params.insert("start", start_date.unwrap().to_string());
    }

    let encoded = form_urlencoded::Serializer::new(String::new()).extend_pairs(&params).finish();
    
    let decoded_secret = BASE64_STANDARD.decode(api_secret).unwrap();


    let signature = get_kraken_signature(&url_path,&encoded,&decoded_secret,&nonce);


    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_str("application/x-www-form-urlencoded").unwrap());
    headers.insert("Accept", HeaderValue::from_str("application/x-www-form-urlencoded").unwrap());
    headers.insert("API-Key", HeaderValue::from_str(api_key).unwrap());
    headers.insert("API-Sign", HeaderValue::from_str(&signature).unwrap());


    let client = reqwest::Client::new();
    let response = client
                            .post(url)
                            .headers(headers)
                            .form(&params)
                            .send()
                            .await
                            .unwrap();

    let text = response.text().await.unwrap();

    let trade_response: TradeResponse = serde_json::from_str(&text).unwrap();

    return Ok(trade_response);

}


#[tokio::main]  // If the function is called often, it is preferable to create the runtime using the runtime builder so the runtime can be reused across calls.
pub async fn fetch_trades_history_kraken() -> Result<Vec<TradeInfo>,Error> {
    let api_key = env::var("KRAKEN_KEY").expect("KRAKEN_KEY not set in .env file");
    let api_secret: String = env::var("KRAKEN_SECRET").expect("KRAKEN_SECRET not set in .env file");
    let mut trade_history : Vec<TradeInfo> = Vec::new();
    let trade_response: TradeResponse = fetch_data(&api_key,&api_secret,None).await?;
    let mut count : u64 = 0;
    let mut last_date : i64= 0;
    if let Some(result) = trade_response.result.as_ref() {
        count = result.count;
        trade_history.extend(result.trades.values().cloned());
        last_date = trade_history.iter().map(|trade| trade.time.floor() as i64).min().unwrap();
        // use offset and pagination not shitty this
    }
    // while count == 50{
    //     let trade_response: TradeResponse = fetch_data(&api_key,&api_secret,Some(last_date)).await?;
    //     if let Some(result) = trade_response.result.as_ref() {
    //         count = result.count;
    //         trade_history.extend(result.trades.values().cloned());
    //         last_date = trade_history.iter().map(|trade| trade.time.floor() as i64).min().unwrap();
    //     }
    // }

    trade_history.sort_by(|a,b| a.time.total_cmp(&b.time));

    return Ok(trade_history);
    
}



#[derive(Debug, Deserialize,Serialize, Clone)]
pub struct TradeInfo {
    pub ordertxid: String,
    pub postxid: String,
    pub pair: String,
    pub time: f64,
    #[serde(rename = "type")]
    pub trade_type: String,
    pub ordertype: String,
    pub price: String,
    pub cost: String,
    pub fee: String,
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


#[derive(Debug, Deserialize,Serialize)]
pub struct TradeHistory {
    pub count: u64, // Total number of entries globally
    pub trades: HashMap<String, TradeInfo>,

}


#[derive(Debug, Deserialize,Serialize)]
pub struct TradeResponse {
    pub error: Vec<String>,
    pub result: Option<TradeHistory>,
}



#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_signature(){
        /* https://docs.kraken.com/api/docs/guides/spot-rest-auth */
        let secret = String::from("kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==");
        let nonce = String::from("1616492376594");
        let payload = String::from("nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25");
        let uri_path = String::from("/0/private/AddOrder");
        let decoded_secret = BASE64_STANDARD.decode(secret).unwrap();

        let signature = get_kraken_signature(&uri_path,&payload,&decoded_secret,&nonce);

        assert_eq!(signature,String::from("4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="));

    }


    #[test]
    fn test_deserialize(){

         let json_data = r#"
        {
        "error": [],
        "result": {
            "trades": {
            "THVRQM-33VKH-UCI7BS": {
                "ordertxid": "OQCLML-BW3P3-BUCMWZ",
                "postxid": "TKH2SE-M7IF5-CFI7LT",
                "pair": "XXBTZUSD",
                "time": 1688667796.8802,
                "type": "buy",
                "ordertype": "limit",
                "price": "30010.00000",
                "cost": "600.20000",
                "fee": "0.00000",
                "vol": "0.02000000",
                "margin": "0.00000",
                "misc": "",
                "trade_id": 40274859,
                "maker": true
            },
            "TCWJEG-FL4SZ-3FKGH6": {
                "ordertxid": "OQCLML-BW3P3-BUCMWZ",
                "postxid": "TKH2SE-M7IF5-CFI7LT",
                "pair": "XXBTZUSD",
                "time": 1688667769.6396,
                "type": "buy",
                "ordertype": "limit",
                "price": "30010.00000",
                "cost": "300.10000",
                "fee": "0.00000",
                "vol": "0.01000000",
                "margin": "0.00000",
                "misc": "",
                "trade_id": 39482674,
                "maker": true
            }
            }
        }
        }
    "#;

    let _trade_response: TradeResponse = serde_json::from_str(json_data).unwrap();
    }
}




