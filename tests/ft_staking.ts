import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { FtStaking } from "../target/types/ft_staking";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from "@solana/spl-token";
import { SystemProgram, Keypair, PublicKey } from "@solana/web3.js";

describe("ft_staking", async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.FtStaking as Program<FtStaking>;
  let connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
  let antCoin = new PublicKey("HHxLugckxjCWDS3epTinHGgcuQHcPLbDNQfvqh3nyxxH"); //dirt token
  let antFood = new PublicKey("HHxLugckxjCWDS3epTinHGgcuQHcPLbDNQfvqh3nyxxH"); //dirt token
  //  2TYV72CtgYXCduE5hheeoux728zHcSyxPQAbhiCNf2Yy
  let owner = Keypair.fromSecretKey(
    Uint8Array.from([113, 63, 93, 213, 68, 178, 22, 189, 136, 49, 33, 174, 196, 213, 238, 242, 164, 106, 9, 180, 15, 3, 238, 80, 159, 127, 118, 18, 231, 206, 240, 93, 21, 168, 99, 61, 85, 242, 222, 187, 12, 44, 91, 158, 122, 83, 103, 113, 125, 136, 28, 83, 108, 248, 78, 219, 197, 250, 38, 187, 70, 109, 130, 194])
  );

  const [globalState, globalStateBump] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("GLOBAL-STATE-SEED"),
      owner.publicKey.toBuffer()
    ],
    program.programId
  );

  const [vault, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("VAULT-SEED")
    ],
    program.programId
  );

  const [minter, minterBump] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("MINTER-STATE-SEED"),
      owner.publicKey.toBuffer()
    ],
    program.programId
  );

  const [antFoodTokenVaultAccount, antFoodTokenVaultAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("TOKEN-VAULT-SEED"),
      antFood.toBuffer()
    ],
    program.programId
  );

  console.log(antFoodTokenVaultAccount.toString());

  const rentSysvar = anchor.web3.SYSVAR_RENT_PUBKEY;
  getTokenBalanceWeb3(connection, antFoodTokenVaultAccount);

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.rpc.initialize(
      owner.publicKey,
      // new anchor.BN(antc_price),
      // new anchor.BN(antc_expo),
      {
        accounts: {
          owner: owner.publicKey,
          globalState,
          vault,
          minter,
          systemProgram: SystemProgram.programId,
          rent: rentSysvar
        },
        signers: [owner]
      }
    )
    console.log("Your transaction signature", tx);


  });

  it("set antc", async () => {
    const tx = await program.rpc.setAntCoin({
      accounts: {
        minterKey: owner.publicKey,
        globalState,
        minter,
        antCoin,
        systemProgram: SystemProgram.programId,
        rent: rentSysvar
      },
      signers: [owner]
    });
  });


  it("set antFood", async () => {
    const tx = await program.rpc.setAntFoodToken({
      accounts: {
        minterKey: owner.publicKey,
        globalState,
        minter,
        newAntFoodToken: antFood,
        systemProgram: SystemProgram.programId,
        rent: rentSysvar
      },
      signers: [owner]
    });
  });

  it("Deposit antFood", async () => {
    const minterAntFoodTokenAccount = await getAssociatedTokenAddress(
      antFood,
      owner.publicKey
    );

    const tx = await program.rpc.depositAntFoodToken(
      new anchor.BN(1_000_000_000_000_000),
      {
        accounts: {
          minterKey: owner.publicKey,
          globalState,
          minter,
          antFoodToken: antFood,
          antFoodTokenVaultAccount,
          minterAntFoodTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: rentSysvar
        },
        signers: [owner]
      });
    console.log(tx);
  });

  // it("stake antc", async () => {

  //   // const antCoin = new PublicKey("FLitGKEPBvBNqPVZbfgRPR5fwcsgSrRv6BDZjxRRFhUC");

  //   const [stakedInfo, stakeInfoBump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [
  //       Buffer.from("STAKED-INFO-SEED"),
  //       owner.publicKey.toBuffer()
  //     ],
  //     program.programId
  //   );

  //   const [antCoinVaultAccount, antCoinVaultAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [
  //       Buffer.from("TOKEN-VAULT-SEED"),
  //       antCoin.toBuffer()
  //     ],
  //     program.programId
  //   );

  //   const userAntCoinAccount = await getAssociatedTokenAddress(
  //     antCoin,
  //     owner.publicKey
  //   );

  //   try {
  //     const tx = await program.rpc.stake(
  //       new anchor.BN(100000000),
  //       {
  //         accounts: {
  //           user: owner.publicKey,
  //           globalState,
  //           stakedInfo,
  //           antCoin,
  //           antCoinVaultAccount,
  //           userAntCoinAccount,
  //           tokenProgram: TOKEN_PROGRAM_ID,
  //           systemProgram: SystemProgram.programId,
  //         },
  //         signers: [owner]
  //       });
  //     console.log("vault coin balance", await getTokenBalanceWeb3(connection, antCoinVaultAccount));

  //     console.log("vault food balance", await getTokenBalanceWeb3(connection, antFoodTokenVaultAccount));
  //     console.log(tx);
  //   } catch (error) {
  //     console.log(error);
  //   }
  // });

  // it("claim", async () => {

  //   // const antCoin = new PublicKey("FLitGKEPBvBNqPVZbfgRPR5fwcsgSrRv6BDZjxRRFhUC");
  //   // const antFood = new PublicKey("4JtesASQCh1ZYDdvCpgpMG5WLxMKyGAVt4tS4QS9L8Np");

  //   const [stakedInfo, stakeInfoBump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [
  //       Buffer.from("STAKED-INFO-SEED"),
  //       owner.publicKey.toBuffer()
  //     ],
  //     program.programId
  //   );

  //   const userAntFoodTokenAccount = await getAssociatedTokenAddress(
  //     antFood,
  //     owner.publicKey
  //   );

  //   try {
  //     const tx = await program.rpc.claim(
  //       {
  //         accounts: {
  //           user: owner.publicKey,
  //           globalState,
  //           stakedInfo,
  //           antFoodToken: antFood,
  //           antFoodTokenVaultAccount,
  //           userAntFoodTokenAccount,
  //           tokenProgram: TOKEN_PROGRAM_ID,
  //           systemProgram: SystemProgram.programId,
  //         },
  //         signers: [owner]
  //       });

  //     console.log("vault food balance", await getTokenBalanceWeb3(connection, antFoodTokenVaultAccount));

  //     console.log(tx);
  //   } catch (error) {
  //     console.log(error);
  //   }
  // });

  // it("unstake antc", async () => {

  //   // const antCoin = new PublicKey("FLitGKEPBvBNqPVZbfgRPR5fwcsgSrRv6BDZjxRRFhUC");
  //   // const antFood = new PublicKey("4JtesASQCh1ZYDdvCpgpMG5WLxMKyGAVt4tS4QS9L8Np");

  //   const [stakedInfo, stakeInfoBump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [
  //       Buffer.from("STAKED-INFO-SEED"),
  //       owner.publicKey.toBuffer()
  //     ],
  //     program.programId
  //   );

  //   const [antCoinVaultAccount, antCoinVaultAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [
  //       Buffer.from("TOKEN-VAULT-SEED"),
  //       antCoin.toBuffer()
  //     ],
  //     program.programId
  //   );

  //   const userAntCoinAccount = await getAssociatedTokenAddress(
  //     antCoin,
  //     owner.publicKey
  //   );

  //   const userAntFoodTokenAccount = await getAssociatedTokenAddress(
  //     antFood,
  //     owner.publicKey
  //   );

  //   try {
  //     const tx = await program.rpc.unstake(
  //       {
  //         accounts: {
  //           user: owner.publicKey,
  //           globalState,
  //           stakedInfo,
  //           antCoin,
  //           antCoinVaultAccount,
  //           userAntCoinAccount,
  //           antFoodToken: antFood,
  //           antFoodTokenVaultAccount,
  //           userAntFoodTokenAccount,
  //           tokenProgram: TOKEN_PROGRAM_ID,
  //           systemProgram: SystemProgram.programId,
  //         },
  //         signers: [owner]
  //       });
  //     console.log("vault coin balance", await getTokenBalanceWeb3(connection, antCoinVaultAccount));

  //     console.log("vault food balance", await getTokenBalanceWeb3(connection, antFoodTokenVaultAccount));

  //     console.log(tx);
  //   } catch (error) {
  //     console.log(error);
  //   }
  // });
});

async function getTokenBalanceWeb3(connection, tokenAccount) {
  const info = await connection.getTokenAccountBalance(tokenAccount);
  if (info.value.uiAmount == null) throw new Error('No balance found');
  console.log('Balance (using Solana-Web3.js): ', info.value.uiAmount);
  return info.value.uiAmount;
}
