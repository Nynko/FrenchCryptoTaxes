/* This file contain the functions related to the calculation of the wallet balances */

use hashbrown::HashMap;

use crate::structs::{Transaction, Wallet};

/* We actually add the balances during the mapping of the data, because this is data. And we update with the data. This
will recaculate the balances. This might not match the data available.
There is two reasons: 
- Knowing the state of a wallet before a specific transactions
- Check the data 
*/
fn _calculate_balances(tx: &Transaction, wallets_copy : &mut HashMap<String, Wallet>){ // Should need some kind of copy of the wallets for not changing the actual data.
    match tx {
        Transaction::Trade { from, to, sold_amount, bought_amount,..} => {
                let [from_wallet,to_wallet] = wallets_copy.get_many_mut([from,to]).unwrap();
                from_wallet.get_mut().balance -= sold_amount;
                to_wallet.get_mut().balance += bought_amount;
        }
        Transaction::Transfer { tx, from, to, amount, cost_basis } => {
            let [from_wallet,to_wallet] = wallets_copy.get_many_mut([from,to]).unwrap();
            from_wallet.get_mut().balance -= amount;
            to_wallet.get_mut().balance += amount;
        },
        Transaction::Deposit { tx, to, amount } => {
            todo!()
        },
        Transaction::Withdrawal { tx, from, amount } => todo!(),
    }
}
