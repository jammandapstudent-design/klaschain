# KlasChain

KlasChain is a peer-to-peer student micro-UBI network where a campus community pools small weekly USDC contributions into a Soroban contract that redistributes the total pool equally to all active members every week.

## Problem & Solution
**Problem:** Students with irregular income from tutoring or gig work face cash flow volatility and often lose small uncollected payments, causing severe financial stress on bad weeks.

**Solution:** A mutual UBI circle where students pool small amounts (e.g., $2) weekly. The contract redistributes the total equally among members, smoothing out income volatility and ensuring everyone gets a baseline payout every week.

## Timeline
Can be demoed in a hackathon setting within 2 minutes.

## Stellar Features Used
- USDC transfers
- Soroban smart contracts
- Trustlines
- Custom tokens (KLAS - conceptual integration for future reputation tracking)

## Vision and Purpose
To create resilient, self-sustaining financial safety nets for students through decentralized mutual aid, rather than relying on top-down charity.

## Prerequisites
- Rust (stable)
- `soroban-cli` v22.0.0 or later

## How to Build
```bash
soroban contract build
