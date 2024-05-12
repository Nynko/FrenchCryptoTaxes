use crate::api::{fetch_history_bitfinex, fetch_history_kraken, Tier, Trade};
pub mod structs;
pub mod functions;
pub mod api;
pub mod parsing;
pub mod errors;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use dotenv::dotenv;
use reqwest::{Error, Response};

// use crate::structs::Transaction;

fn main() {
    dotenv().ok();
    let response = fetch_history_kraken(Tier::Intermediate).unwrap();
    println!("{:?}",response);
    println!("{:?}",response.len());

    ()
}
