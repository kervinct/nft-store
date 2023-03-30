const anchor = require('@project-serum/anchor');
const assert = require("assert");
const {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} = require("@solana/spl-token");
const {
  getTokenAccount,
  createMint,
  createTokenAccount,
} = require("./utils");
const { token } = require("@project-serum/anchor/dist/cjs/utils");

describe('nftstore', () => {
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Nftstore;

  const nftAmount = new anchor.BN(1);
  let nftMintAccount = null;
  let nftMint = null;
  let creatorKeypair = null;
  let sellerKeypair = null;
  let sellerTokenAccount = null;
  let customerKeypair = null;
  let customerTokenAccount = null;

  it('Initialize the state-of-the-world', async () => {
    // 默认账号，创建mint
    nftMintAccount = await createMint(provider);
    nftMint = nftMintAccount.publicKey;
    // console.log("nft: ", nftMint.toString());

    // 主办方
    creatorKeypair = anchor.web3.Keypair.generate();
    // 卖方
    sellerKeypair = anchor.web3.Keypair.generate();
    // console.log("seller: ", authorityKeypair.publicKey.toString());
    // 买方
    customerKeypair = anchor.web3.Keypair.generate();
    // console.log("customer: ", customerKeypair.publicKey.toString());

    await provider.connection.requestAirdrop(
      creatorKeypair.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 100,
    );
    
    await provider.connection.requestAirdrop(
      sellerKeypair.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 10,
    );

    await provider.connection.requestAirdrop(
      customerKeypair.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 50,
    );
      
    // 为卖方创建NFT
    sellerTokenAccount = await createTokenAccount(
      provider,
      nftMint,
      sellerKeypair.publicKey,
    );
    await nftMintAccount.mintTo(
      sellerTokenAccount,
      provider.wallet.publicKey,
      [],
      nftAmount.toString(),
    );

    // 为买方创建NFT账户
    customerTokenAccount = await createTokenAccount(
      provider,
      nftMint,
      customerKeypair.publicKey,
    );

    // 验证发放
    let authority_nft_account = await getTokenAccount(
      provider,
      sellerTokenAccount,
    );
    assert.ok(authority_nft_account.amount.eq(nftAmount));
  });

  let storeName;
  storeName = "slope";

  it("Initialize the Store", async() => {
    const [storeAccount, storeAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(storeName)],
      program.programId,
    );

    await program.rpc.initializeStore(
      storeName,
      storeAccountBump,
      {
        accounts: {
          creator: creatorKeypair.publicKey,
          storeAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [creatorKeypair],
      }
    );
  });

  it("Initialize the Record", async () => {
    let bumps = new RecordBumps();

    const [storeAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(storeName)],
      program.programId,
    );

    const [recordAccount, recordAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_record_account")],
      program.programId,
    );
    bumps.recordAccount = recordAccountBump;
    // console.log("store record account: ", recordAccount.toString());

    const [recordTokenAccount, recordTokenAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_account")],
      program.programId,
    )
    bumps.recordTokenAccount = recordTokenAccountBump;
    // console.log("record nft account: ", recordTokenAccount.toString());

    await program.rpc.initializeRecord(
      bumps,
      {
        accounts: {
          authority: sellerKeypair.publicKey,
          nftMint,
          recordTokenAccount,
          recordAccount,
          storeAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [sellerKeypair],
      }
    );

    let record_token_account = await getTokenAccount(provider, recordTokenAccount);
    assert.ok(record_token_account.amount.eq(new anchor.BN(0)));
  });

  it("Sell user's nft on store", async () => {

    const [storeAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(storeName)],
      program.programId,
    );

    const [recordAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_record_account")],
      program.programId,
    );

    const [recordTokenAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_account")],
      program.programId,
    )

    let store_account = await program.account.storeAccount.fetch(storeAccount);

    let listener = null;
    let [event, slot] = await new Promise((resolve, _reject) => {
      listener = program.addEventListener("LaunchEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.sellNft(
        new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 10),
        new anchor.BN(10), {
        accounts: {
          authority: sellerKeypair.publicKey,
          authorityTokenAccount: sellerTokenAccount,
          recordTokenAccount,
          recordAccount,
          storeAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [sellerKeypair],
      });
    });
    await program.removeEventListener(listener);

    assert.ok(slot > 0);
    assert.ok(event.seller.toString() == sellerKeypair.publicKey.toString());
    assert.ok(event.mint.toString() == nftMint.toString());
    assert.ok(event.price.toNumber() === 10000000000);
    assert.ok(event.rate === 10);
    assert.ok(event.label === "sell_nft");

    let authority_nft_account = await getTokenAccount(
      provider,
      sellerTokenAccount,
    );
    assert.ok(authority_nft_account.amount.eq(new anchor.BN(0)));
    let record_nft_account = await getTokenAccount(
      provider,
      recordTokenAccount,
    );
    assert.ok(record_nft_account.amount.eq(new anchor.BN(1)));

    let record_account = await program.account.recordAccount.fetch(recordAccount);
    assert.ok(record_account.price.eq(new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 10)));
    assert.ok(record_account.onSale);

    let res = await provider.connection.getParsedAccountInfo(sellerKeypair.publicKey);
    console.log(res);
  });

  it("Buy nft", async() => {
    
    const [storeAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(storeName)],
      program.programId,
    );

    const [recordAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_record_account")],
      program.programId,
    );

    const [recordTokenAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_account")],
      program.programId,
    );

    let record_account = await program.account.recordAccount.fetch(recordAccount);
    let store_account = await program.account.storeAccount.fetch(storeAccount);

    const [soldRecord] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("sold_record"), new anchor.BN(0).toArray('le', 4)],
      program.programId,
    );

    console.log("before buy nft");
    let owner_account = await provider.connection.getParsedAccountInfo(creatorKeypair.publicKey);
    console.log(owner_account.value.lamports);

    let listener = null;
    let [event, slot] = await new Promise((resolve, _reject) => {
      listener = program.addEventListener("SoldEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.buyNft({
        accounts: {
          authority: customerKeypair.publicKey,
          receiver: record_account.seller,
          holder: store_account.owner,
          soldRecord,
          authorityTokenAccount: customerTokenAccount,
          recordTokenAccount,
          recordAccount,
          storeAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        },
        signers: [customerKeypair],
      });
    });
    await program.removeEventListener(listener);

    console.log(new Date(event.createdAt.toNumber() * 1000));
    assert.ok(slot > 0);
    assert.ok(event.seller.toString() == sellerKeypair.publicKey.toString());
    assert.ok(event.mint.toString() == nftMint.toString());
    assert.ok(event.customer.toString() == customerKeypair.publicKey.toString());
    assert.ok(event.index === 0);
    assert.ok(event.price.toNumber() === 10000000000);
    assert.ok(event.rate === 10);
    assert.ok(event.label === "buy_nft");

    console.log("after buy nft");
    owner_account = await provider.connection.getParsedAccountInfo(creatorKeypair.publicKey);
    console.log(owner_account.value.lamports);
    let customer_token_account = await getTokenAccount(provider, customerTokenAccount);
    assert.ok(customer_token_account.amount.eq(new anchor.BN(1)));

    let sold_record_account = await program.account.soldRecord.fetch(soldRecord);
    assert.ok(sold_record_account.price.eq(new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 10)));
  });

  it("Redeem nft", async() => {

    const [storeAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(storeName)],
      program.programId,
    );

    const [recordAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_record_account")],
      program.programId,
    );

    const [recordTokenAccount] = await anchor.web3.PublicKey.findProgramAddress(
      [nftMint.toBuffer(), Buffer.from("nft_account")],
      program.programId,
    );

    let store_account = await program.account.storeAccount.fetch(storeAccount);
    console.log("before sell");
    let record_account_info = await provider.connection.getParsedAccountInfo(recordAccount);
    console.log(record_account_info.value.lamports);
    let customer_account = await provider.connection.getParsedAccountInfo(customerKeypair.publicKey);
    console.log(customer_account.value.lamports);
    // 挂售
    await program.rpc.sellNft(
      new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 20),
      new anchor.BN(1), {
      accounts: {
        authority: customerKeypair.publicKey,
        authorityTokenAccount: customerTokenAccount,
        recordTokenAccount,
        recordAccount,
        storeAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [customerKeypair],
    });

    let record_token_account = await getTokenAccount(provider, recordTokenAccount);
    assert.ok(record_token_account.amount.eq(new anchor.BN(1)));
    let authority_nft_account = await getTokenAccount(provider, customerTokenAccount);
    assert.ok(authority_nft_account.amount.eq(new anchor.BN(0)));
    console.log("before redeem");
    record_account_info = await provider.connection.getParsedAccountInfo(recordAccount);
    console.log(record_account_info.value.lamports);
    customer_account = await provider.connection.getParsedAccountInfo(customerKeypair.publicKey);
    console.log(customer_account.value.lamports);

    let listener = null;
    // 赎回
    let [event, slot] = await new Promise((resolve, _reject) => {
      listener = program.addEventListener("RedeemEvent", (event, slot) => {
        resolve([event, slot]);
      });
      program.rpc.redeemNft({
        accounts: {
          authority: customerKeypair.publicKey,
          authorityTokenAccount: customerTokenAccount,
          recordTokenAccount,
          recordAccount,
          storeAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [customerKeypair],
      });
    });
    await program.removeEventListener(listener);

    assert.ok(slot > 0);
    assert.ok(event.redeem.toString() == customerKeypair.publicKey.toString());
    assert.ok(event.mint.toString() == nftMint.toString());
    assert.ok(event.label === "redeem_nft");

    record_token_account = await getTokenAccount(provider, recordTokenAccount);
    assert.ok(record_token_account.amount.eq(new anchor.BN(0)));
    authority_nft_account = await getTokenAccount(provider, customerTokenAccount);
    assert.ok(authority_nft_account.amount.eq(new anchor.BN(1)));

    let record_account = await program.account.recordAccount.fetch(recordAccount);
    // console.log(record_account);
    console.log("after redeem");
    record_account_info = await provider.connection.getParsedAccountInfo(recordAccount);
    console.log(record_account_info.value.lamports);
    customer_account = await provider.connection.getParsedAccountInfo(customerKeypair.publicKey);
    console.log(customer_account.value.lamports);
  });

  function RecordBumps() {
    this.recordAccount;
    this.recordTokenAccount;
  };
});
