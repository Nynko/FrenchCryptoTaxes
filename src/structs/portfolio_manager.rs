use serde::{Deserialize, Serialize};

/* This manager is used for associating the price of a wallet at a time t, associated with a transaction
This allow to save the history of price of the asset for calculating the full value of the portfolio without
having to call API again often.

If only one wallet is added, then we only have to call one API for this specific Wallet.
*/
#[derive(Serialize, Deserialize)]
pub struct PortfolioManager{
    
}