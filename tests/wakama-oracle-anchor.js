const anchor = require("@coral-xyz/anchor");

describe("wakama-oracle-anchor", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.WakamaOracleAnchor;

  it("initializes an asset and pushes an oracle update", async () => {
    // Mint utilisé uniquement comme seed stable
    const assetMint = anchor.web3.Keypair.generate().publicKey;
    const oracleAuthority = provider.wallet.publicKey;

    const [assetPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rwa-asset"), assetMint.toBuffer()],
      program.programId
    );

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
    await program.methods
      .pushOracleUpdate(new anchor.BN(10_000), new anchor.BN(5))
      .accounts({
        asset: assetPda,
        oracle: oracleAuthority,
      })
      .rpc();

    const asset = await program.account.rwaAsset.fetch(assetPda);

    console.log("Program ID:", program.programId.toBase58());
    console.log("Asset PDA:", assetPda.toBase58());
    console.log("total_points:", asset.totalPoints.toString());
    console.log("total_batches:", asset.totalBatches.toString());
    console.log("invested_usdc:", asset.investedUsdc.toString());
  });
});
