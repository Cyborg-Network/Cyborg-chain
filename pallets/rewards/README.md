# Rewards Pallet ( pallet-rewards )

## Overview

A subsystem to allow for an automatic `reward` process, whereby a reward will be given to `providers` for providing a service or completing a task.

In this version, the pallet offers foundational incentives to providers for maintaining consistent network connectivity. Rewards are recalculated hourly in the database based on random connectivity assessments by the edge connect pallet, with daily payout distribution.

### Terminology

- **Reward:** A reward is a payment made to a provider for providing a service or completing a task, in a timely manner.
- **Provider:** A provider is an account that is eligible to receive a reward.

## Interface

### Dispatchable Functions

- `check_provider_eligibility` - Check if a provider is eligible to receive a reward.
- `reward_provider` - Reward a provider for providing a service or completing a task.
