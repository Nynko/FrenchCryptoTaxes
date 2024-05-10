use crate::structs::{CurrentPortfolio, SimpleDecimal, Transaction, TransactionBase};  

/* Calculate the french "plus-value" to fill the form 2086

plus_value =  prix_cession - (acquisition_pf_net * prix_cession / valeur_pf )
*/
pub fn calculate_tax(tx: Transaction)-> SimpleDecimal{
    match tx {
        Transaction::Transfer { amount, price_eur, pf, .. } =>  {
            let sell_price: SimpleDecimal = amount * price_eur; 
            return _calculate_tax(sell_price,&pf);
        },
        Transaction::Trade { bought_amount, bought_price_eur, pf, .. } =>  {
            let sell_price: SimpleDecimal = bought_amount * bought_price_eur; 
            return _calculate_tax(sell_price,&pf);
        },
        _ => SimpleDecimal::new(0,0)
    }
}


pub fn _calculate_tax(sell_price: SimpleDecimal, pf: &Option<CurrentPortfolio>) -> SimpleDecimal{
    if pf.is_none(){
        return SimpleDecimal::new(0,0);
    }
    let portfolio = pf.as_ref().unwrap();
    return sell_price - calculate_weigted_price(sell_price, portfolio.pf_cost_basis, portfolio.pf_total_value);
}

/* (acquisition_pf_net * prix_cession / valeur_pf ) */
pub fn calculate_weigted_price(sell_price: SimpleDecimal, current_cost_basis: SimpleDecimal, pf_total_value: SimpleDecimal) -> SimpleDecimal{
    return current_cost_basis * sell_price / pf_total_value;
}



/* Calculate the cost_basis, here "acquisition_pf_net" */
pub fn calculate_cost_basis(tx : &mut Transaction, current_pf: CurrentPortfolio ) -> CurrentPortfolio{
    match tx {
        Transaction::Transfer { tx, from: _, to: _, amount, price_eur, pf } => {
            *pf = Some(current_pf.clone());
            return calculate_new_portfolio(tx,&current_pf,*amount,*price_eur);
        }
        Transaction::Trade { tx, from:_, to:_, sold_amount: _, bought_amount,bought_price_eur, pf } => {
            *pf = Some(current_pf.clone());
            return calculate_new_portfolio(tx,&current_pf,*bought_amount,*bought_price_eur)
            },
        _ => current_pf // ignoring the fiat deposit and withdrawal as they don't change the cost basis, they are here for accounting
    }
    
}

/* Calculate new Portfolio: if the transaction is taxable the new cost basis will change, otherwise only the fee might change it */
fn calculate_new_portfolio(
    tx: &TransactionBase,
    current_pf: &CurrentPortfolio,
    amount: u64,
    price_eur: SimpleDecimal,
) -> CurrentPortfolio {
    let current_cost_basis = current_pf.pf_cost_basis;
    let current_total_cost = current_pf.pf_total_cost;
    let pf_total_value = current_pf.pf_total_value;

    let fee = if tx.fee.is_some() && tx.fee_price.is_some() {tx.fee.unwrap() * tx.fee_price.unwrap()} else {SimpleDecimal::new(0, 2)};
    let mut cost_basis_adjustment :SimpleDecimal = SimpleDecimal::new(0,2);
    let mut pf_value_adjustment : SimpleDecimal = fee;
    if tx.is_taxable {
        // Selling of Crypto - Taxable event
        let sell_price: SimpleDecimal = amount * price_eur; 
        let weigted_price = calculate_weigted_price(sell_price, current_cost_basis, pf_total_value);
        cost_basis_adjustment = weigted_price;
        pf_value_adjustment = sell_price + fee
    }

    return CurrentPortfolio {
        pf_cost_basis: current_cost_basis - cost_basis_adjustment + fee,
        pf_total_cost: current_total_cost + fee,
        pf_total_value:pf_total_value - pf_value_adjustment
    }
}




#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::structs::{Currency, Owner, Platform, Wallet, WalletBase};

    use super::*;

    use super::calculate_cost_basis;



    fn get_pf(cost_basis: u64,total_cost: u64,total_value: u64) -> CurrentPortfolio {
        return CurrentPortfolio {
            pf_cost_basis: SimpleDecimal::new(cost_basis,2),
            pf_total_cost: SimpleDecimal::new(total_cost,2),
            pf_total_value: SimpleDecimal::new(total_value,2)
        };
    }

    #[test]
    fn simple_transfer_with_fee() {
        let current_pf = get_pf(500,500,1000);
        let currency = Currency{
            id: String::from("test"),
            name: String::from("bitcoin"),
            symbol: String::from("BTC"),
            decimals: 8
        };

        let platform = Platform {
            id: String::from("test"),
            name: String::from("Binance"),
        };

        let from = Wallet::Crypto(WalletBase{
            currency:currency,
            platform: platform,
            owner:  Owner::User,
            balance:1,
            discriminator: None
        });

        let to = from.clone();

        let mut tx = Transaction::Transfer { tx: TransactionBase{
            id: String::from("test"),
            fee: Some(SimpleDecimal::new(10,2)),
            fee_price:Some(SimpleDecimal::new(1,0)),
            timestamp: Utc::now(),
            is_taxable:false
        }, from: from, to: to, amount: 1, price_eur: SimpleDecimal::new(64000,2), pf: None };

        let new_pf = calculate_cost_basis(&mut tx,current_pf);

        assert_eq!(new_pf.pf_total_cost,SimpleDecimal::new(500 + 10,2) );
        assert_eq!(new_pf.pf_total_value, SimpleDecimal::new(1000-10,2));
        assert_eq!(new_pf.pf_cost_basis, SimpleDecimal::new(500 + 10,2));


    }

}