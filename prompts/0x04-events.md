Add events to the contract. This should log an event whenever a user deposits or withdraws and other major functions.

Also add a public function that can be used to get a snapshot of the current depositors. I understand this may work only up to a certain amount of users. It should return an array of Address -> Token balance.

Ideally token balance would update inline with the users share of USDC in the pool. So if they deposit $100 into the vault they get 100 vault tokens back. If they accrue 5% interest over a year there token balance is now 105. This is similar to how stETH works on Ethereum where the user doesn't need to do anything and they can just watch their token balance increase. If this is possible please implement it as well.