use csv::ReaderBuilder;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
struct Transaction {
    id: String,
    date_utc: String,
    transaction_type: String,
    tag: String,
    from_wallet: String,
    from_wallet_id: String,
    from_amount: String,
    from_currency: String,
    to_wallet: String,
    to_wallet_id: String,
    to_amount: String,
    to_currency: String,
    fee_amount: String,
    fee_currency: String,
    net_worth_amount: String,
    net_worth_currency: String,
    fee_worth_amount: String,
    fee_worth_currency: String,
    net_value: String,
    fee_value: String,
    value_currency: String,
    deleted: String,
    from_source: String,
    to_source: String,
    negative_balances: String,
    missing_rates: String,
    missing_cost_basis: String,
    synced_to_accounting_at_utc: String,
    tx_src: String,
    tx_dest: String,
    tx_hash: String,
    description: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let mut file = File::open("transactions.csv")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse CSV data
    let mut rdr = ReaderBuilder::new().from_reader(contents.as_bytes());
    // for result in rdr.deserialize::<Transaction>() {
    //     let record = result?;
    //     println!("{:?}", record);
    // }

    Ok(())
}
