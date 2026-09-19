/**
 * ShiftPay Escrow — Frontend-Backend Entegrasyon Sözleşmesi (Contract Interface & ABI)
 * Arkadaşının Frontend'de Doğrudan Çağıracağı Fonksiyonlar ve Tipler
 */

export const SHIFTPAY_TESTNET_CONFIG = {
  networkPassphrase: "Test SDF Network ; September 2015",
  rpcUrl: "https://soroban-testnet.stellar.org",
  // Testnet'e deploy edildikten sonra üretilen contract adresi buraya gelir:
  contractId: "CDV3G7DHQEVUAMKRKK6UXVAUPLY277RRJVZNYAQX44OK4VIW5XUWFPII",
  anchorUrl: "https://tr-mock-anchor.fly.dev",
  defindexVaultId: "CCJWW63WRWZASW7YIGWASHZVOKMDEKQ557CJOHRA5X3PG5KDPHWVITD5"
};

/**
 * 1. İŞVEREN EKRANI İÇİN FONKSİYONLAR
 */
export const EmployerActions = {
  /**
   * İşveren bütçe/faktoring limiti kilitler (DeFindex Vault'ta getiri üretir)
   * @param {string} tokenAddress - USDC veya TRY_CREDIT adresi
   * @param {bigint} amount - Kilitlenecek tutar (Örn: 20000_0000000n)
   * @param {boolean} autoStakeDefindex - true (Atıl bütçeyi DeFindex getiri havuzuna yatır)
   */
  deposit: "deposit(employer: Address, token_address: Address, amount: i128, lock_period_seconds: u64, auto_stake_defindex: bool)",

  /**
   * İşçi için günlük hak ediş ücreti belirler
   * @param {string} workerAddress - İşçinin Stellar cüzdan adresi
   * @param {bigint} dailyWage - Günlük ücret (Örn: 1000_0000000n)
   */
  setWage: "set_wage(employer: Address, worker: Address, daily_wage: i128)",

  /**
   * Hatalı/sahte vardiyaya itiraz eder (İşçiye otomatik borç yazar)
   */
  disputeCheckout: "dispute_checkout(employer: Address, worker: Address, shift_id: BytesN<32>, spent_amount: i128)"
};

/**
 * 2. İŞÇİ EKRANI İÇİN FONKSİYONLAR
 */
export const WorkerActions = {
  /**
   * İşçi işe geldiğinde amirin QR kodunu okutup vardiyayı başlatır
   * @param {string} shiftIdHex - 32-byte hex (Örn: '0x01...32')
   */
  checkIn: "check_in(worker: Address, shift_id: BytesN<32>)",

  /**
   * Vardiya bittiğinde check-out yapar (Hakediş üretilir, geçmiş borç varsa otomatik kesilir)
   */
  checkOut: "check_out(employer: Address, worker: Address)",

  /**
   * Anlaşmalı esnafta (kahve, yemek) anında harcar (Sıfır takas riski)
   * @param {string} merchantAddress - Esnafın cüzdan adresi
   * @param {bigint} amount - Harcanacak tutar
   */
  spendAtMerchant: "spend_at_merchant(worker: Address, merchant: Address, shift_id: BytesN<32>, amount: i128)",

  /**
   * Vadesi gelen hak edişi Anchor ile TL olarak IBAN'ına çeker
   */
  withdrawToIban: "withdraw(worker: Address, shift_id: BytesN<32>, token_address: Address)"
};

/**
 * 3. ESNAF / MAĞAZA EKRANI İÇİN FONKSİYONLAR
 */
export const MerchantActions = {
  /**
   * Esnaf biriken hak ediş alacaklarını kendi şirket TL IBAN'ına çeker
   */
  withdrawRevenue: "merchant_withdraw(merchant: Address, token_address: Address, amount: i128)"
};

/**
 * 4. OKUMA (GETTER) FONKSİYONLARI (Arayüzde Bakiye Göstermek İçin)
 */
export const QueryActions = {
  getEmployerVault: "get_employer_vault(employer: Address) -> EmployerVault",
  getWorkerClaim: "get_worker_claim(shift_id: BytesN<32>) -> WorkerClaim",
  getWorkerDebtState: "get_worker_debt_state(worker: Address) -> WorkerDebtState",
  getMerchantBalance: "get_merchant_balance(merchant: Address) -> i128"
};
