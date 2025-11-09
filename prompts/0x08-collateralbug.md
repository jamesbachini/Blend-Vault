I currently have the request type set to 0 and 1 to supply assets to Blend in contracts/src/lib.rs

However I have been looking at code that uses these values for a similar yield strategy:

pub enum RequestType {
    SupplyCollateral = 2,
    WithdrawCollateral = 3,
}

I also looked at the get_positions function for this smart contract and a personal account where I've deposited funds to blend to earn interest.

For the current contract it returns:
{
  "collateral": {},
  "liabilities": {},
  "supply": {
    "1": "9802639"
  }
}

For my personal account it returns:

{
  "collateral": {
    "1": "123456"
  },
  "liabilities": {},
  "supply": {}
}

I think we need to change the request types in our contract. We also need to change the calculation for total_assets

DO NOT REDEPLOY THE CONTRACT, I WILL DO THAT MANUALLY VIA deploy.sh

Step 1. Check this is correct
Step 2. Change the request types in lib.rs
Step 3. Update unit tests to reflect this
Step 4. Update the frontend
Step 5. Check work and update Claude.md

I want to remove any use of shares in the frontend and just use USDC amounts where possible. I believe we can use the withdraw function rather than redeem to withdraw an asset value rather than a share value in the contract, let's do this in the frontend.
