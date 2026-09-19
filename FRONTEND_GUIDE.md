# ShiftPay — Frontend Entegrasyon Rehberi (Arkadaşın İçin)

Merhaba! Backend ve Soroban akıllı sözleşme mimarisi tamamlandı ve test edildi. 
Frontend'i geliştirirken doğrudan bu fonksiyonları ve akışları kullanabilirsin.

---

## 🚀 1. Kullanıcı Girişi & Cüzdan Bağlantısı (Wallets)

İki seçeneğin var (ikisini de backend'de hazırladık):
1. **Kolay Sosyal Giriş (Privy):** İşçi veya esnaf için "Google ile Giriş Yap" butonu koyabilirsin. 12 kelime gerekmeden arka planda Stellar cüzdanı açılır (`scripts/privy_auth.js`).
2. **Kripto Cüzdanı (Stellar Wallets Kit):** Klasik cüzdan bağlamak isteyenler için Albedo, Freighter, LOBSTR modalını açarsın (`scripts/wallets_kit_integration.js`).

---

## 📋 2. Ekrana Göre Çağrılacak Kontrat Fonksiyonları

Ayrıntılı ABI ve parametreler: `shared/contract_interface.js` dosyasında hazır.

### 🏢 A. İşveren Ekranı (Employer Dashboard)
* **Bütçe Kilitleme:** `deposit(employer, token_address, amount, lock_period_seconds, auto_stake_defindex: true)`
  * *İpucu:* `auto_stake_defindex: true` gönderildiğinde para DeFindex Vault'ta beklerken getiri (faiz) üretir.
* **İşçiye Günlük Ücret Tanımlama:** `set_wage(employer, worker, daily_wage)`
* **Vardiyaya İtiraz Etme:** `dispute_checkout(employer, worker, shift_id, spent_amount)`
* **Kilitli Bütçeyi Okuma:** `get_employer_vault(employer)`

### 👷 B. İşçi Terminali (Worker Mobile Screen)
* **QR ile İşe Giriş (Check-in):** `check_in(worker, shift_id)`
* **Vardiya Bitirme (Check-out):** `check_out(employer, worker)`
  * Kontrat otomatik olarak hakedişi oluşturur. Varsa geçmiş borcu otomatik keser.
* **Mağazada Kahve/Yemek Harcaması:** `spend_at_merchant(worker, merchant, shift_id, amount)`
  * İşçi mağazanın QR kodunu okutur, alacak sıfır takas riskiyle anında mağazaya geçer.
* **Vadesi Gelen Parayı Bankaya Çekme:** `withdraw(worker, shift_id, token_address)`
  * Ardından SEP-24 Anchor popup'ı tetiklenir ve para IBAN'a FAST ile gider (`scripts/anchor_integration.js`).
* **Borç ve Bakiye Durumu:** `get_worker_debt_state(worker)` & `get_worker_claim(shift_id)`

### ☕ C. Mağaza / Esnaf Ekranı (Merchant POS)
* **Mağazanın QR Kodu:** Ekranda mağazanın `merchantAddress`'ini içeren bir QR gösterilir.
* **Biriken Ciroyu Görme:** `get_merchant_balance(merchant)`
* **Ciroyu Şirket Banka Hesabına Çekme:** `merchant_withdraw(merchant, token_address, amount)`
  * SEP-24 Anchor ile doğrudan ticari IBAN'a TL geçer.

---

## 🔗 3. Testnet Bağlantı Bilgileri
* **RPC URL:** `https://soroban-testnet.stellar.org`
* **Network Passphrase:** `Test SDF Network ; September 2015`
* **Anchor URL:** `https://testanchor.stellar.org`
