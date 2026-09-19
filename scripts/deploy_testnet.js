/**
 * ShiftPay Testnet Deployer & Faucet Funder
 * Deploys shiftpay_escrow.wasm directly to Stellar Testnet using @stellar/stellar-sdk
 */

const fs = require("fs");
const path = require("path");
const { Keypair, rpc, Networks, TransactionBuilder, Operation, Address } = require("@stellar/stellar-sdk");

const RPC_URL = "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE = Networks.TESTNET;
const server = new rpc.Server(RPC_URL);

async function fundWithFriendbot(publicKey) {
  console.log(`\n1. Friendbot ile Testnet Hesabı Fonlanıyor: ${publicKey}`);
  const response = await fetch(`https://friendbot.stellar.org?addr=${encodeURIComponent(publicKey)}`);
  if (!response.ok) {
    throw new Error(`Friendbot fonlama hatası: ${response.statusText}`);
  }
  console.log(`✅ Hesap başarıyla fonlandı (10,000 Testnet XLM).`);
}

async function deployShiftPay() {
  console.log(`======================================================`);
  console.log(`🚀 SHIFTPAY ESCROW — STELLAR TESTNET DEPLOYMENT`);
  console.log(`RPC: ${RPC_URL}`);
  console.log(`======================================================`);

  // 1. Deployer hesabı oluştur ve fonla
  const deployer = Keypair.random();
  console.log(`Deployer Açık Anahtarı: ${deployer.publicKey()}`);
  console.log(`Deployer Gizli Anahtarı: ${deployer.secret()}`);

  await fundWithFriendbot(deployer.publicKey());

  // 2. WASM dosyasını oku
  const wasmPath = path.join(__dirname, "../contracts/shiftpay_escrow/shiftpay_escrow.wasm");
  const wasmBytes = fs.readFileSync(wasmPath);
  console.log(`\n2. WASM Dosyası Okundu: ${wasmBytes.length} bytes`);

  // 3. Hesap durumunu çek
  const account = await server.getAccount(deployer.publicKey());

  // 4. Upload WASM işlemi oluştur
  console.log(`3. WASM Kodu Testnet'e Yükleniyor (Upload WASM Bytecode)...`);
  const uploadTx = new TransactionBuilder(account, { fee: "1000000", networkPassphrase: NETWORK_PASSPHRASE })
    .addOperation(Operation.uploadContractWasm({ wasm: wasmBytes }))
    .setTimeout(60)
    .build();

  const preparedUploadTx = await server.prepareTransaction(uploadTx);
  preparedUploadTx.sign(deployer);
  
  const uploadSendRes = await server.sendTransaction(preparedUploadTx);
  console.log(`İşlem Gönderildi: ${uploadSendRes.hash}. Sonuç bekleniyor...`);
  
  let uploadTxRes = await server.getTransaction(uploadSendRes.hash);
  while (uploadTxRes.status === "NOT_FOUND") {
    await new Promise(r => setTimeout(r, 2000));
    uploadTxRes = await server.getTransaction(uploadSendRes.hash);
  }

  if (uploadTxRes.status !== "SUCCESS") {
    throw new Error(`Upload WASM başarısız: ${JSON.stringify(uploadTxRes)}`);
  }

  const wasmHash = uploadTxRes.returnValue.bytes().toString("hex");
  console.log(`✅ WASM Yüklendi! WASM Hash: ${wasmHash}`);

  // 5. Create Contract Instance (Deploy)
  console.log(`\n4. Sözleşme Örneği Oluşturuluyor (Create Contract Instance)...`);
  const freshAccount = await server.getAccount(deployer.publicKey());
  
  const createTx = new TransactionBuilder(freshAccount, { fee: "1000000", networkPassphrase: NETWORK_PASSPHRASE })
    .addOperation(Operation.createCustomContract({
      address: Address.fromString(deployer.publicKey()),
      wasmHash: Buffer.from(wasmHash, "hex")
    }))
    .setTimeout(60)
    .build();

  const preparedCreateTx = await server.prepareTransaction(createTx);
  preparedCreateTx.sign(deployer);

  const createSendRes = await server.sendTransaction(preparedCreateTx);
  console.log(`İşlem Gönderildi: ${createSendRes.hash}. Sonuç bekleniyor...`);

  let createTxRes = await server.getTransaction(createSendRes.hash);
  while (createTxRes.status === "NOT_FOUND") {
    await new Promise(r => setTimeout(r, 2000));
    createTxRes = await server.getTransaction(createSendRes.hash);
  }

  if (createTxRes.status !== "SUCCESS") {
    throw new Error(`Create contract başarısız: ${JSON.stringify(createTxRes)}`);
  }

  const contractId = Address.fromScAddress(createTxRes.returnValue.address()).toString();
  console.log(`\n======================================================`);
  console.log(`🎉 TEBRİKLER! SÖZLEŞME STELLAR TESTNET'TE CANLI:`);
  console.log(`CONTRACT ID: ${contractId}`);
  console.log(`STELLAR EXPERT LINKİ: https://stellar.expert/explorer/testnet/contract/${contractId}`);
  console.log(`======================================================`);

  return { contractId, wasmHash, deployerPublicKey: deployer.publicKey() };
}

if (require.main === module) {
  deployShiftPay().catch(err => {
    console.error("Hata oluştu:", err);
  });
}

module.exports = { deployShiftPay };
