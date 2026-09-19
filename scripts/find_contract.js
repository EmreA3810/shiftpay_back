const { StrKey } = require("@stellar/stellar-sdk");

async function findExactContract() {
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
  const buffer = Buffer.from(data.result.resultMetaXdr, "base64");
  
  // A contract ID is a 32-byte sha256 / contract id with standard strkey prefix
  // Let's slide 32 bytes and test each
  const candidates = new Set();
  for (let i = 0; i <= buffer.length - 32; i++) {
    const slice = buffer.subarray(i, i + 32);
    try {
      const encoded = StrKey.encodeContract(slice);
      // Valid contract strkey has length 56 and starts with C
      if (encoded.length === 56 && encoded.startsWith("C") && !encoded.startsWith("CAAAAAAA")) {
        candidates.add(encoded);
      }
    } catch(e) {}
  }
  for (const c of candidates) {
    console.log("Candidate:", c);
  }
}

findExactContract();
