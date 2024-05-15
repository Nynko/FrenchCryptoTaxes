use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/* Representation of the total portfolio cost basis at a specific point in time (before the associated transaction).
This is calculated iteratively after having transformed the data.
*/
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCostBasis {
    pub pf_cost_basis: Decimal, // Cost basis of the portfolio in euro (acquisition NET)
    pub pf_total_cost: Decimal, // Cost basis of the portfolio in euro (acquisition BRUTE)
}
