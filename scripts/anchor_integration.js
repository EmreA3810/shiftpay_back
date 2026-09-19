/**
 * ShiftPay Anchor Client (SEP-24 Interactive Flow & SEP-6 Direct)
 * Pro Hackathon 2026 - Rise In x Stellar
 * 
 * Demonstrates:
 * 1. Employer Deposit (TRY -> on-chain TRY_CREDIT)
 * 2. Worker Withdrawal (Vadesi gelen hakediş -> Türk Lirası IBAN)
 * 3. Merchant Withdrawal (Mağazanın kazandığı stabil kripto / hakediş -> Türk Lirası IBAN)
 */

const TESTNET_ANCHOR_URL = "https://testanchor.stellar.org";

class StellarAnchorService {
  constructor(anchorUrl = TESTNET_ANCHOR_URL) {
    this.anchorUrl = anchorUrl;
  }

  /**
   * 1. İşveren TL Yatırma (On-Ramp: TRY -> on-chain TRY_CREDIT)
   */
  async depositEmployerBudget(employerAddress, amountTry) {
    console.log(`\n======================================================`);
    console.log(`🏦 [SEP-24 ANCHOR] İŞVEREN TL YATIRMA (ON-RAMP)`);
    console.log(`İşveren Cüzdanı: ${employerAddress}`);
    console.log(`Yatırılan Tutar: ₺${amountTry.toLocaleString('tr-TR')} TRY`);
    
    const popupUrl = `${this.anchorUrl}/sep24/interactive?asset_code=TRY&account=${employerAddress}&amount=${amountTry}`;
    console.log(`🔗 Anchor Hosted Popup URL: ${popupUrl}`);
    console.log(`📲 İşveren şirket banka hesabından FAST ile Anchor IBAN'ına transfer yaptı.`);
    console.log(`✅ Onaylandı: ${amountTry} TRY_CREDIT on-chain varlığı sözleşmeye aktarıldı.`);
    
    return { success: true, asset: "TRY_CREDIT", amount: amountTry, txType: "DEPOSIT_SUCCESS" };
  }

  /**
   * 2. İşçinin Hakedişini TL Olarak IBAN'ına Çekmesi (Worker Off-Ramp)
   */
  async withdrawWorkerWage(workerAddress, iban, amountTry) {
    console.log(`\n======================================================`);
    console.log(`👷 [SEP-24 ANCHOR] İŞÇİ TL ÇEKİMİ (OFF-RAMP)`);
    console.log(`İşçi Cüzdanı: ${workerAddress}`);
    console.log(`Hedef IBAN: ${iban}`);
    console.log(`Çekilecek Hak Ediş: ₺${amountTry.toLocaleString('tr-TR')} TRY`);

    console.log(`1. Soroban Sözleşmesi 'withdraw()' / 'settle_matured_claim()' çağrıldı.`);
    console.log(`2. SEP-24 Anchor çekim uç noktası tetiklendi (asset_code=TRY).`);
    console.log(`✅ FAST 7/24 Transferi Başarılı: ₺${amountTry} işçinin banka hesabına geçti.`);

    return { success: true, beneficiary: "worker", iban, amount: amountTry, status: "PAID_TO_IBAN" };
  }

  /**
   * 3. Mağazanın / Esnafın Kazandığı Parayı Kendi TL Hesabına Çekmesi (Merchant Off-Ramp)
   * Mağaza kazandığı stabilcoin/alacakları elinde tutmak zorunda değildir; doğrudan bankasına çeker.
   */
  async withdrawMerchantRevenue(merchantAddress, merchantIban, amountTry) {
    console.log(`\n======================================================`);
    console.log(`☕ [SEP-24 ANCHOR] MAĞAZA / ESNAF TL ÇEKİMİ (OFF-RAMP)`);
    console.log(`Mağaza/Esnaf Cüzdanı: ${merchantAddress}`);
    console.log(`Mağaza Ticari IBAN: ${merchantIban}`);
    console.log(`Çekilen Ciro/Tahsilat: ₺${amountTry.toLocaleString('tr-TR')} TRY`);

    console.log(`1. Soroban Sözleşmesi 'merchant_withdraw()' çağrıldı.`);
    console.log(`2. SEP-24 Anchor ödeme köprüsü devreye girdi.`);
    console.log(`✅ Mağaza Kapanışı (Settlement): ₺${amountTry} mağazanın banka hesabına ödendi.`);

    return { success: true, beneficiary: "merchant", iban: merchantIban, amount: amountTry, status: "MERCHANT_SETTLED" };
  }
}

// Doğrulama Testi
if (require.main === module) {
  (async () => {
    const anchor = new StellarAnchorService();
    
    // 1. İşveren bütçe yatırır
    await anchor.depositEmployerBudget("GCVWBPBYHXKCNPMR2PVSDZN3DFW37BH2YEQFMVVKHHDHDTDIPLN5FXBE", 20000);

    // 2. İşçi hakedişini çeker
    await anchor.withdrawWorkerWage("GBWORKER4X79...STELLAR", "TR33 0006 1005 1982 0001 2345 67", 600);

    // 3. Mağaza kazandığı ciroyu kendi banka hesabına çeker
    await anchor.withdrawMerchantRevenue("GBMERCHANT88...STELLAR", "TR66 0001 5001 8888 0009 8765 43", 400);
  })();
}

module.exports = { StellarAnchorService };
