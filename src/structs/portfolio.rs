use super::SimpleDecimal;



/* Representation of the total portfolio at a specific point in time (before the associated transaction) */
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct CurrentPortfolio{
    pub pf_total_value: SimpleDecimal, // Portfolio total value in euro
    pub pf_cost_basis: SimpleDecimal, // Cost basis of the portfolio in euro (acquisition NET)
    pub pf_total_cost: SimpleDecimal  // Cost basis of the portfolio in euro (acquisition BRUTE)
}
