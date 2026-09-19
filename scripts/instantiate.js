const { Keypair, rpc, Networks, TransactionBuilder, Operation, Address } = require("@stellar/stellar-sdk");

const RPC_URL = "https://soroban-testnet.stellar.org";
const server = new rpc.Server(RPC_URL);
const WASM_HASH = "bc6e15d95905246feb10b626d76164f1ac37692b54a3311489d4712209730e66";

// Previous deployer credentials
const deployer = Keypair.fromSecret("SC5MGYZE2MYL2B2DCEM24MTVPFEULCF4E2FEESKMC7MNTTLQ2JDXGEDB");

async function instantiateContract() {
  console.log(`Deployer: ${deployer.publicKey()}`);
  console.log(`Using uploaded WASM Hash: ${WASM_HASH}`);

  const account = await server.getAccount(deployer.publicKey());

  const createTx = new TransactionBuilder(account, { fee: "2000000", networkPassphrase: Networks.TESTNET })
    .addOperation(Operation.createCustomContract({
      address: Address.fromString(deployer.publicKey()),
      wasmHash: Buffer.from(WASM_HASH, "hex")
    }))
    .setTimeout(60)
    .build();

  console.log("Simulating / preparing transaction...");
  const prepared = await server.prepareTransaction(createTx);
  prepared.sign(deployer);

  console.log("Submitting createContract transaction...");
  const sendRes = await server.sendTransaction(prepared);
  console.log(`Submitted tx: ${sendRes.hash}`);

  let txRes = await server.getTransaction(sendRes.hash);
  while (txRes.status === "NOT_FOUND") {
    await new Promise(r => setTimeout(r, 2000));
    txRes = await server.getTransaction(sendRes.hash);
  }

  console.log("Raw Tx Status:", txRes.status);
  if (txRes.status !== "SUCCESS") {
    console.error("Tx failed:", txRes);
    return;
  }

  const contractAddress = Address.fromScAddress(txRes.returnValue.address()).toString();
  console.log("\n========================================================");
  console.log("🎉 SHIFTPAY ESCROW SÖZLEŞMESİ BAŞARIYLA INSTANTIATE EDİLDİ!");
  console.log(`CONTRACT ID: ${contractAddress}`);
  console.log(`STELLAR EXPERT: https://stellar.expert/explorer/testnet/contract/${contractAddress}`);
  console.log("========================================================");
}

instantiateContract().catch(console.error);
