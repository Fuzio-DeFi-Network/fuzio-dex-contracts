# Fuzio-Staking

This contract is a contract that allows staking (bonding) the LP tokens that you get when depositing liquidity a fuzio-pool and obtain one or multiple tokens as mining rewards.

# Instantiation

The contract can be instantiated with the following messages

```
{
    "lp_token_contract": "<LP_TOKEN_CONTRACT>",
    "reward_token": [{"native": "<DENOM_1>"}, ... {"cw20": "<DENOM_N>"}],
    "distribution_schedule": [[<distribution_schedule_1>,...<distribution_schedule_n>]],
    "lock_duration": "<BOND_TIME_IN_SECONDS>"
}
```

The token rewards can be either `native` for tokens tracked by the bank module (including IBC assets or created with token factory) or `cw20` for cw20 tokens. `native` tokens have a denom string and `cw20` tokens have a contract address.

Distribution schedule is a Vec<(u64, u64, Uint128)> array for each reward token. It indicates how many tokens will be given about between the period provided. If the rewards are constantly given out, this array with only have 1 element for each reward token.

# Messages

### Cw20ReceiveMsg

The LP tokens will be sent to the contract accompanied by a bond message so that the contract can immediately bond the tokens and the user will start receiving rewards.

### Unbond

Allows a user to unbond liquidity from the pool.

### Withdraw

Allows a user to withdraw liquidity from the pool.

### Redeem

Allows a user to redeem his rewards from the pool.

### UpdateConfig (Admin only)

Allows the admin to modify the distribution schedule of pool rewards.

### UpdateAdmin (Admin only)

Modifies admin of contract.

### UpdateTokenContract (Admin only)

Modifies the LP token contract and the array of rewards.

### UpdateTokensAndDistribution (Admin only)

Modifies the LP token contract, the array of rewards and distribution schedule.

### UpdateLockDuration (Admin only)

Modifies the bond time for the pool.

