# French Crypto Taxes : An open source project for calculating your crypto taxes

> English version below

Ce projet est en cours de développement, la première version devrait arriver avant les impôts de 2025. 

Si vous aimeriez participer à ce projet, n'hésitez pas à me contacter !


------------

This project is currently under development, with the first version due to arrive before the French taxes of 2025. 
If you'd like to take part in this project, don't hesitate to contact me!


## Quick introduction: Why this project ?

Filling out French taxes (but even all crypto taxes in general) isn't easy... in fact, it's getting harder and harder to do it manually as the number of cryptocurrencies you hold and the number of exchanges you use increases. This is because, in France, we use a “portfolio” representation of all your holdings, and need the price of the entire portfolio at any given time you make a taxable action. In my opinion, you need a tool to do this... but the market for such tools is not only expensive, but given the way French taxes are run, it sometimes becomes absurd.

You can literally invest €100, withdraw it, and have to pay tax on that event when you've made no gain “locally”. Using tools that promise you “one-click” efficiency (which is often false) also prevents you from knowing how to manage your portfolio from a French tax point of view. For example, you may have to pay 100 or 200 euros for a tool to fill in your tax return correctly, even if you only have a few taxable events. But since we need the whole history and the whole portfolio to calculate French taxes, we have to pay for these tools...

By creating this tool, I hope to do two things:
- Educate on how French taxes really works
- Giving the ability to handle your taxes for free (especially for a period of low gains).

In the future, I hope this could lead to a community-driven open-source tool for all crypto taxes too !

## Implementation and notes

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
