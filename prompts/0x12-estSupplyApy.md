I believe the blend UI calculates APR by calling the following smart contract function:

get_reserve(USDC_CONTRACT_ADDRESS)

This returns the following data:
{
  "asset": "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
  "config": {
    "c_factor": 9500000,
    "decimals": 7,
    "enabled": true,
    "index": 1,
    "l_factor": 9500000,
    "max_util": 9500000,
    "r_base": 300000,
    "r_one": 400000,
    "r_three": 50000000,
    "r_two": 1200000,
    "reactivity": 20,
    "supply_cap": "2000000000000000",
    "util": 8000000
  },
  "data": {
    "b_rate": "1072799171533",
    "b_supply": "44831235792545",
    "backstop_credit": "764496026",
    "d_rate": "1107697781106",
    "d_supply": "37101864781356",
    "ir_mod": "23921867",
    "last_time": "1762943352"
  },
  "scalar": "10000000"
}

What I want to know is how they calculate the APR supply rate and can you add this to our frontend alongside our own vault TVL and blends yieldbox pool TVL.

So add to the top of the frontend page:
APR 21.38% / Vault TVL $1.02 / Pool TVL $21.41m

Maybe the APR should be calculated slightly higher as we will be autocompounding the BLND emmissions daily?

TVL can just query balances of USDC for the two contracts, no need to make it too complicated.

Current rates are as follows, use these to check maths with the above figures but work out the actual rate dynamically so it changes when the YieldBox pool APR changes:
USDC interest earned 20.37%
BLND emissions earned 1.01%
Net interest earned 21.38%


Some code from the github repo at: https://github.com/blend-capital/blend-ui/


// src/components/asset/InterestGraph.tsx
const targetUtil = reserve.config.util / 1e7;
const maxUtil = reserve.config.max_util / 1e7;
const currentUtil = reserve.getUtilizationFloat();
const currentIRModFloat = FixedMath.toFloat(
reserve.data.interestRateModifier,
reserve.irmodDecimals
);

let dataPoints: { util: number; apr: number }[] = [];
let defaultDataPoints: { util: number; apr: number }[] = [];
let utilizationRates = [];
for (let i = 0; i <= (showMore ? 100 : maxUtil * 100); i++) {
utilizationRates.push(i / 100);
}
utilizationRates = utilizationRates.concat([currentUtil, targetUtil]);
utilizationRates.sort((a, b) => a - b);
dataPoints = [
...utilizationRates.map((utilRate) => ({
    util: utilRate,
    apr: estimateInterestRate(utilRate, currentIRModFloat, reserve, backstopTakeRate),
})),
];
defaultDataPoints = [
...utilizationRates.map((utilRate) => ({
    util: utilRate,
    apr: estimateInterestRate(utilRate, 1, reserve, backstopTakeRate),
})),
];
const maxAPR =
dataPoints.length > 0 && defaultDataPoints.length > 0
    ? Math.max(
        dataPoints[dataPoints.length - 1].apr,
        defaultDataPoints[defaultDataPoints.length - 1].apr
    )
    : 1;



---------------
// src/utils/math.ts

export function estimateEmissionsApr(
  emissionsPerAssetPerYear: number,
  backstopToken: BackstopToken,
  assetPrice: number
): number {
  const usdcPerBlnd =
    FixedMath.toFloat(backstopToken.usdc, 7) /
    0.2 /
    (FixedMath.toFloat(backstopToken.blnd, 7) / 0.8);
  return (emissionsPerAssetPerYear * usdcPerBlnd) / assetPrice;
}

/**
 * Estimate the interest rate for a reserve given a utilization ratio
 * @param util utilization ratio as a float
 * @param ir_mod interest rate modifier as a float
 * @param reserve The reserve to estimate the interest rate for
 * @param backstopTakeRate The backstop take rate as a fixed point number
 */
export function estimateInterestRate(
  util: number,
  ir_mod: number,
  reserve: Reserve,
  backstopTakeRate: bigint
): number {
  const RATE_SCALAR = FixedMath.toFixed(1, reserve.rateDecimals);
  // setup reserve with util and ir_mod
  let ir_resData = new ReserveData(
    RATE_SCALAR,
    RATE_SCALAR,
    FixedMath.toFixed(ir_mod, reserve.irmodDecimals),
    FixedMath.toFixed(util, reserve.config.decimals),
    FixedMath.toFixed(1, reserve.config.decimals),
    BigInt(0),
    0
  );
  let ir_reserve =
    reserve.rateDecimals === 9
      ? new ReserveV1('', '', reserve.config, ir_resData, undefined, undefined, 0, 0, 0, 0, 0)
      : new ReserveV2('', '', reserve.config, ir_resData, undefined, undefined, 0, 0, 0, 0, 0);
  ir_reserve.setRates(backstopTakeRate);
  return ir_reserve.borrowApr;
}