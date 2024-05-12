use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/* Representation of the total portfolio at a specific point in time (before the associated transaction)
Importante notes !!

The pf_total_value should be set depending on the gloabl value of the portfolio before each transaction (at least each taxable one).
It can be caculated from the price of all wallets at an instant t.
The issue is getting price at instant t may take time (calling API). We want to get that information before actually treating the transaction,
when we only want it for taxable events.
*/
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct CurrentPortfolio {
    pub pf_total_value: Decimal, // Portfolio total value in euro --> only necessary to know for taxable events
    pub pf_cost_basis: Decimal,  // Cost basis of the portfolio in euro (acquisition NET)
    pub pf_total_cost: Decimal,  // Cost basis of the portfolio in euro (acquisition BRUTE)
}
