const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const wasmPath = path.join(__dirname, "../contracts/shiftpay_escrow/shiftpay_escrow.wasm");
const wasmBytes = fs.readFileSync(wasmPath);
const wasmHash = crypto.createHash("sha256").update(wasmBytes).digest("hex");
console.log("WASM SHA256 HASH:", wasmHash);
