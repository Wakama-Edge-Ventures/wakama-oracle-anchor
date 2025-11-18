# Wakama Oracle Anchor – RWA Sample Program

Devnet RWA demo program used for Wakama Oracle Milestone 1 with Solana Foundation.

- Program name: `wakama-oracle-anchor`
- Program ID (devnet): `93eL55wjf62Pw8UPsKS8V7b9efk28UyG8C74Vif2gMNR`
- Cluster: `devnet`
- Explorer:  
  https://explorer.solana.com/address/93eL55wjf62Pw8UPsKS8V7b9efk28UyG8C74Vif2gMNR?cluster=devnet

Upgrade authority (Phantom wallet):  
`GYdgCBzz9Phzvdh8dTz9VRB8qyjVbA6GscYPKTrGBczR`

---

## 1. What this program does

This Anchor program exposes a minimal but realistic RWA asset contract connected to the Wakama Oracle pipeline. It is designed to be easy to read and extend, while being close to a real-world RWA use case.

The program:

- Tracks a single RWA asset (for example a tokenized farm, plot, or coop basket).
- Aggregates oracle "points" and batches coming from the Wakama Oracle (sensor data, coop batches, etc.).
- Maintains a simple lifecycle status: `Pending`, `Active`, `Redeemed`, `Defaulted`.
- Tracks an "invested USDC volume" value for the asset (proxy for RWA investment volume).
- Emits events for every important operation, so that a dashboard or Solscan-style explorer can display on-chain activity clearly.

The program does not handle token transfers or custody directly. It is a clean on-chain state machine plus events, used as the RWA example backing Milestone 1 and the dashboard.

---

## 2. Instructions (entrypoints)

All instructions are defined in:

`programs/wakama-oracle-anchor/src/lib.rs`

### 2.1 `initialize_asset(asset_mint, oracle_authority)`

Initializes a new RWA asset PDA:

- PDA seeds: `["rwa-asset", asset_mint]`
- Sets:
  - `authority` = `payer`
  - `oracle_authority` = provided pubkey
  - `asset_mint` = provided mint/pubkey
  - `status` = `Pending`

Accounts:

- `asset` – PDA `Account<RwaAsset>` (init, payer = `payer`)
- `payer` – `Signer`
- `asset_mint` – `UncheckedAccount` (used as a stable seed only)
- `system_program`

### 2.2 `push_oracle_update(points_delta, batch_count)`

Used by the Wakama Oracle publisher to aggregate oracle data into the asset:

- Only callable by `oracle_authority`.
- Adds `points_delta` and `batch_count` to `RwaAsset`.
- Updates `last_oracle_update_slot` and `updated_at`.
- Emits an `OracleUpdate` event.

Accounts:

- `asset` – PDA `RwaAsset` (mut)
- `oracle` – `Signer` (must equal `oracle_authority`)

### 2.3 `set_status(new_status: u8)`

Lifecycle management for the RWA asset:

- Allowed statuses:
  - `Pending` (0)
  - `Active` (1)
  - `Redeemed` (2)
  - `Defaulted` (3)
- Only `authority` can update the status.

Accounts:

- `asset` – PDA `RwaAsset` (mut)
- `authority` – `Signer` (must equal `asset.authority`)

### 2.4 `record_investment(amount_usdc)`

Simple RWA investment tracker:

- Adds `amount_usdc` to `asset.invested_usdc`.
- Does not perform any token transfer (that can be handled by a separate treasury or off-chain process).
- Emits an `InvestmentRecorded` event that includes the `investor` pubkey.

Accounts:

- `asset` – PDA `RwaAsset` (mut)
- `authority` – `Signer`
- `investor` – `UncheckedAccount` (used for logging in the event only)

---

## 3. Accounts

### 3.1 `RwaAsset` account

```rust
pub struct RwaAsset {
    pub authority: Pubkey,
    pub oracle_authority: Pubkey,
    pub asset_mint: Pubkey,
    pub bump: u8,
    pub status: RwaStatus,
    pub total_points: u64,
    pub total_batches: u64,
    pub invested_usdc: u64,
    pub last_oracle_update_slot: u64,
    pub created_at: i64,
    pub updated_at: i64,
}


3.2 RwaStatus enum
pub enum RwaStatus {
    Pending  = 0,
    Active   = 1,
    Redeemed = 2,
    Defaulted = 3,
}

4. Build and deploy
4.1 Prerequisites

Rust and Cargo

Solana CLI (Agave) 2.1.x, configured on devnet:

solana config set --url https://api.devnet.solana.com


Anchor CLI 0.31.1

Node.js and npm

4.2 Build

From the repository root:

anchor build

4.3 Deploy to devnet

Anchor.toml already contains the program ID:

[programs.devnet]
wakama-oracle-anchor = "93eL55wjf62Pw8UPsKS8V7b9efk28UyG8C74Vif2gMNR"


Deploy:

anchor deploy --provider.cluster devnet

5. Quickstart: call initialize_asset and push_oracle_update

Create tests/rwa-sample.js:

const anchor = require("@coral-xyz/anchor");

describe("wakama-oracle-anchor – RWA sample", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.WakamaOracleAnchor;

  it("initializes an asset and pushes an oracle update", async () => {
    // Mint used only as a stable seed; no SPL constraints here.
    const assetMint = anchor.web3.Keypair.generate().publicKey;

    // For this demo, oracle authority = wallet
    const oracleAuthority = provider.wallet.publicKey;

    const [assetPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rwa-asset"), assetMint.toBuffer()],
      program.programId
    );

    console.log("Program ID:", program.programId.toBase58());
    console.log("Asset PDA:", assetPda.toBase58());
    console.log("Asset mint:", assetMint.toBase58());
    console.log("Oracle authority:", oracleAuthority.toBase58());

    // 1) initialize_asset
    await program.methods
      .initializeAsset(assetMint, oracleAuthority)
      .accounts({
        asset: assetPda,
        payer: provider.wallet.publicKey,
        assetMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // 2) push_oracle_update
    const pointsDelta = new anchor.BN(10_000);
    const batchCount = new anchor.BN(5);

    await program.methods
      .pushOracleUpdate(pointsDelta, batchCount)
      .accounts({
        asset: assetPda,
        oracle: oracleAuthority,
      })
      .rpc();

    const asset = await program.account.rwaAsset.fetch(assetPda);

    console.log("RWA asset state:", {
      authority: asset.authority.toBase58(),
      oracleAuthority: asset.oracleAuthority.toBase58(),
      assetMint: asset.assetMint.toBase58(),
      status: asset.status,
      totalPoints: asset.totalPoints.toString(),
      totalBatches: asset.totalBatches.toString(),
      investedUsdc: asset.investedUsdc.toString(),
      lastOracleUpdateSlot: asset.lastOracleUpdateSlot.toString(),
      createdAt: asset.createdAt.toString(),
      updatedAt: asset.updatedAt.toString(),
    });
  });
});


Run the test:

anchor test


This will:

Initialize a demo asset on devnet.

Push an oracle update.

Print the on-chain state in the test logs.

© 2025 Wakama.farm – supported by Solana Foundation
