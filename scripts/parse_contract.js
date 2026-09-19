const { StrKey } = require("@stellar/stellar-sdk");
const fs = require("fs");

async function parseContractId() {
  const res = await fetch("https://soroban-testnet.stellar.org", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getTransaction",
      params: { hash: "ad08833baa5c961d5144437829aea4078df73a34690dc152508042de2bd7213b" }
    })
  });
  const data = await res.json();
  const metaBase64 = data.result.resultMetaXdr;
  const buffer = Buffer.from(metaBase64, "base64");
  
  // Search for 32-byte contract id or match strkey
  // In Stellar Testnet, contract addresses start with 'C' and are encoded with StrKey.encodeContract
  console.log("Transaction Result Status:", data.result.status);
  
  // Extract created contract address from meta buffer
  // ScAddress contract type tag is usually 1, followed by 32 bytes
  for (let i = 0; i < buffer.length - 36; i++) {
    // contract key header in LedgerEntryChange
    if (buffer[i] === 0x00 && buffer[i+1] === 0x00 && buffer[i+2] === 0x00 && buffer[i+3] === 0x06) {
      const candidateBytes = buffer.subarray(i + 4, i + 36);
      try {
        const contractId = StrKey.encodeContract(candidateBytes);
        console.log("Found Contract ID candidate:", contractId);
      } catch (e) {}
    }
  }
}

parseContractId();
