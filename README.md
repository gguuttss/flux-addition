# Flux addition

This blueprint fixes a couple of small issues with the Flux protocol:

## 1. Overflow on redemption
In VERY VERY VERY extreme cases, there could be a decimal overflow at redemption, causing redemptions to be halted (no money is lost, just redemptions become impossible). This can only happen if the first in line CDP to be redeemed is incredibly small. So to fix that, this blueprint adds the ability to immediately close any CDP that's incredibly small (without going through a redemption).

## 2. Mint with usd
Flux is not seeing much use as of late. This means, there is little to none fUSD on the market. If people want to close loans, they need to be able to get fUSD. In order to do that, this blueprint adds the ability to mint fUSD using centralized stables (such as hUSDC) at a fixed rate. For instance, when properly configured it could be possible to mint 1 fUSD with 1.01 hUSDC.

## 3. Partial liquidations
Someone could create an incredibly large loan, larger than the stability pool would be able to liquidate. Then, we can use the panic liquidations from the main contract, but this is usually not economically viable (price impact when selling the collateral and people often don't even have enough USD to be able to panic liquidate). Therefore, this blueprint adds the ability to liquidate loans in pieces using an accepted centralized stablecoin (partial liquidations), **but only if ** the stability pool does not contain enough fUSD to liquidate the loan.
