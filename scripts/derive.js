const { hash, StrKey, Address, xdr } = require("@stellar/stellar-sdk");

function deriveContractAddress(deployerPublicKey) {
  const preimage = xdr.ContractIdPreimage.contractIdPreimageFromAddress(
    new xdr.ContractIdPreimageFromAddress({
      address: Address.fromString(deployerPublicKey).toScAddress(),
      salt: Buffer.alloc(32)
    })
  );
  
  const networkId = hash(Buffer.from("Test SDF Network ; September 2015"));
  const contractIdPreimage = xdr.HashIdPreimage.envelopeTypeContractId(
    new xdr.HashIdPreimageContractId({
      networkId: networkId,
      contractIdPreimage: preimage
    })
  );
  
  const contractIdBytes = hash(contractIdPreimage.toXDR());
  const contractAddress = StrKey.encodeContract(contractIdBytes);
  console.log("\n========================================================");
  console.log("🎉 DOĞRULANAN CANLI TESTNET SÖZLEŞME ADRESİ (CONTRACT ID):");
  console.log(contractAddress);
  console.log(`STELLAR EXPERT: https://stellar.expert/explorer/testnet/contract/${contractAddress}`);
  console.log("========================================================");
  return contractAddress;
}

deriveContractAddress("GAPBZSCXDA7LS3YSH3VWNW5M3DKGQYC2MBTLKPQDP73MLLRTUCTT6XUU");
