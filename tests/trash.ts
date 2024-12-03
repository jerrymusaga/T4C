import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Trash4coin } from "../target/types/trash4coin";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { expect } from "chai";

describe("trash4coin", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Trashy4coin as Program<Trash4coin>;
  const authority = Keypair.generate();
  const user = Keypair.generate();
  let nftConfigPda: PublicKey;
  let nftMint: PublicKey;
  let redeemableMint: PublicKey;
  let userNftTokenAccount: PublicKey;
  let authorityRedeemableTokenAccount: PublicKey;
  let userRedeemableTokenAccount: PublicKey;

  before(async () => {
    // Airdrop SOL to authority and user
    await provider.connection.requestAirdrop(
      authority.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.requestAirdrop(
      user.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
  });

  it("Initializes the program", async () => {
    const [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("nft_config")],
      program.programId
    );
    nftConfigPda = configPda;

    await program.methods
      .initialize(5)
      .accounts({
        authority: authority.publicKey,
        nftConfig: nftConfigPda,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();

    const nftConfig = await program.account.nftConfig.fetch(nftConfigPda);
    expect(nftConfig.authority.toString()).to.equal(
      authority.publicKey.toString()
    );
    expect(nftConfig.maxNftTypes).to.equal(5);
    expect(nftConfig.nftTypes).to.be.empty;
  });

  it("Adds an NFT type", async () => {
    await program.methods
      .addNftType("Test NFT", "TNFT", "https://example.com/nft")
      .accounts({
        authority: authority.publicKey,
        nftConfig: nftConfigPda,
      })
      .signers([authority])
      .rpc();

    const nftConfig = await program.account.nftConfig.fetch(nftConfigPda);
    expect(nftConfig.nftTypes).to.have.lengthOf(1);
    expect(nftConfig.nftTypes[0].name).to.equal("Test NFT");
    expect(nftConfig.nftTypes[0].symbol).to.equal("TNFT");
    expect(nftConfig.nftTypes[0].uri).to.equal("https://example.com/nft");
  });

  it("Sets reward amount for NFT type", async () => {
    await program.methods
      .setRewardAmount(0, new anchor.BN(100))
      .accounts({
        authority: authority.publicKey,
        nftConfig: nftConfigPda,
      })
      .signers([authority])
      .rpc();

    const nftConfig = await program.account.nftConfig.fetch(nftConfigPda);
    expect(nftConfig.nftTypes[0].rewardAmount.toString()).to.equal("100");
  });

  it("Creates redeemable token", async () => {
    redeemableMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9
    );

    authorityRedeemableTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      authority,
      redeemableMint,
      authority.publicKey
    );

    await program.methods
      .createRedeemableToken(new anchor.BN(1000000000))
      .accounts({
        authority: authority.publicKey,
        redeemableMint: redeemableMint,
        redeemableTokenAccount: authorityRedeemableTokenAccount,
        nftConfig: nftConfigPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();

    const tokenAccount = await provider.connection.getTokenAccountBalance(
      authorityRedeemableTokenAccount
    );
    expect(tokenAccount.value.uiAmount).to.equal(1);
  });

  it("Mints an NFT", async () => {
    nftMint = Keypair.generate().publicKey;
    const metadataAddress = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_PROGRAM_ID.toBuffer(),
        nftMint.toBuffer(),
      ],
      program.programId
    )[0];
    const masterEditionAddress = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_PROGRAM_ID.toBuffer(),
        nftMint.toBuffer(),
        Buffer.from("edition"),
      ],
      program.programId
    )[0];

    userNftTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      user,
      nftMint,
      user.publicKey
    );

    await program.methods
      .mintNft(0, new anchor.BN(1))
      .accounts({
        minter: user.publicKey,
        mint: nftMint,
        tokenAccount: userNftTokenAccount,
        metadata: metadataAddress,
        masterEdition: masterEditionAddress,
        nftConfig: nftConfigPda,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_PROGRAM_ID, // This should be the actual Metaplex Token Metadata Program ID
      })
      .signers([user])
      .rpc();

    const tokenAccount = await provider.connection.getTokenAccountBalance(
      userNftTokenAccount
    );
    expect(tokenAccount.value.uiAmount).to.equal(1);
  });

  it("Redeems and burns NFT", async () => {
    userRedeemableTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      user,
      redeemableMint,
      user.publicKey
    );

    const metadataAddress = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_PROGRAM_ID.toBuffer(),
        nftMint.toBuffer(),
      ],
      program.programId
    )[0];

    await program.methods
      .redeemAndBurnNft(new anchor.BN(1))
      .accounts({
        user: user.publicKey,
        nftMint: nftMint,
        nftTokenAccount: userNftTokenAccount,
        redeemableMint: redeemableMint,
        redeemableTokenAccount: authorityRedeemableTokenAccount,
        userRedeemableTokenAccount: userRedeemableTokenAccount,
        authority: authority.publicKey,
        metadata: metadataAddress,
        nftConfig: nftConfigPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([user])
      .rpc();

    const nftTokenAccount = await provider.connection.getTokenAccountBalance(
      userNftTokenAccount
    );
    expect(nftTokenAccount.value.uiAmount).to.equal(0);

    const userRedeemableBalance =
      await provider.connection.getTokenAccountBalance(
        userRedeemableTokenAccount
      );
    expect(userRedeemableBalance.value.uiAmount).to.equal(100 / 1e9); // 100 / 1e9 because the reward amount is 100 and decimals is 9
  });
});
