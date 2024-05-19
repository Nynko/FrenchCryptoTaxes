/* This file is used to check for missing Fiat to Crypto transactions, which should induce a cost_basis and create issue 
when calculating the taxes, as a cost should exist at some point in time.

The user can choose to add the set of missing transactions as manual transactions as a result (this imply a potentially "wrong" local_cost basis) but 
it will still produce better tax results.

The way we are checking it is by iterating over the transactions to see if any transaction Crypto-Crypto is missing a previous Trade FiatToCrypto.

More complex example: Tx(Fiat to Crypto 1, value= 1), Tx(Crypto  1 to Crypto2, value =2 ).. here we are missing  a Tx(Fiat to Crypto 1, value = 1).

The way we can detect a missing trade FTC (FiatToCrypto) is by iterating over the transactions and update a copy of the wallet balance. if at any time we have a negative 
balance, it means we are missing a trade and we need to create a trade correcting this value or at least raise a warning if the implication on taxes.
*/
