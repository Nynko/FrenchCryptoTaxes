use crate::structs::{GlobalCostBasis, Portfolio, Transaction};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/* Calculate the french "plus-value" to fill the form 2086

capital_gain = net_proceeds - (portfolio_cost_basis * net_proceeds / portfolio_total_value)
where net_proceeds = gross_sale_price - selling_fees

In French :
plus_value =  prix_cession - (acquisition_pf_net * prix_cession / valeur_pf )

The transaction must be taxable, otherwise it will panic !
*/
pub fn calculate_tax_gains(tx: &Transaction, portfolio: &Portfolio, cost_basis: &GlobalCostBasis) -> Decimal {
    match tx {
        Transaction::Transfer {
            amount,
            to,from,
            ..
        }
        | Transaction::Trade {
            sold_amount: amount,
            to, from,
            ..
        } => {
            let pf_total_value = portfolio.pf_total_value;
            let sell_price: Decimal = Decimal::from(*amount) * from.price_eur;
            let fee = to.fee.unwrap_or(dec!(0)) * to.price_eur + from.fee.unwrap_or(dec!(0)) * from.price_eur;
            return _calculate_tax(sell_price, fee, cost_basis, pf_total_value);
        }
        _ => dec!(0),
    }
}

pub fn _calculate_tax(
    sell_price: Decimal,
    total_fee: Decimal,
    cost_basis: &GlobalCostBasis,
    pf_total_value: Decimal,
) -> Decimal {
    let net_proceeds = sell_price - total_fee;
    return net_proceeds - calculate_weigted_price(net_proceeds, cost_basis.pf_cost_basis, pf_total_value);
}

/* portfolio_cost_basis * net_proceeds / portfolio_total_value */
pub fn calculate_weigted_price(
    net_proceeds: Decimal,
    current_cost_basis: Decimal,
    pf_total_value: Decimal,
) -> Decimal {
    return current_cost_basis * net_proceeds / pf_total_value;
}
