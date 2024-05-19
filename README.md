# French Crypto Taxes : An open source project for calculating your crypto taxes

> English version below

Ce projet est en cours de développement, la première version devrait arriver avant les impôts de 2025. 

Si vous aimeriez participer à ce projet, n'hésitez pas à me contacter !

------------

This project is currently under development, with the first version due to arrive before the French taxes of 2025. 
If you'd like to take part in this project, don't hesitate to contact me!


Current implementation:
- Kraken API and partial mapping
- Global structure 
- Calculating global_cost basis
- Calculating full portfolio for each taxable transaction (total value of the crypto portfolio)
- Saving data (through what I called managers) as MessagePack with serde serialization and deserialization, for now all is in memory.

Todo: 
- [ ] Checking missing trade
- [ ] CLI interaction / or front end implementation
- [ ] More tests implementations
- [ ] Exporting Formulaire 2086 for a specific year
- [ ] Other API implementation
- [ ] Parsing of csv files
- [ ] ....
