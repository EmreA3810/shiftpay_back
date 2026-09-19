# StreamRail: Üretim Kalitesinde Sistem ve Backend Mimarisi (Production Architecture)
**Stellar & Soroban Smart Contracts | Genesis Track**

---

## 1. Yönetici Özeti & Değer Önerisi (Executive Summary)

**StreamRail**, etkinlik/saha personeli, mevsimlik işçiler ve serbest çalışanlar (gig economy) için tasarlanmış; **faktoring/kredi teminatı, kriptografik QR check-in ve kapalı devre mağaza harcamasını** birleştiren gerçek zamanlı bir hak ediş protokolüdür.

### Geleneksel Bankacılıktan Temel Farklar (Neden Blokzincir?)
1. **İşverenin Sıcak Nakit Bağlamaması (Factoring / Credit Line Integration):**
   * İşveren cebinden peşin 100.000 TL çıkarmak zorunda değildir. Anlaşmalı fintek/faktoring kurumu, Stellar üzerinde işveren adına `TRY_CREDIT` (SEP-41 uyumlu kredi limiti) tanımlar. Sözleşme bu kredi limitini teminat kabul ederek kilitler.
2. **Kriptografik Sahtecilik Önleme (Ed25519 QR Doğrulama):**
   * İşçi evden sahte check-in yapamaz. Saha amirinin/işverenin özel anahtarıyla (private key) anlık üretilen, süre kısıtlı (time-bound) ve tek kullanımlık (nonce replay-protected) QR kodları akıllı sözleşme üzerinde `env.crypto().ed25519_verify()` ile doğrulanır.
3. **Sıfır Takas Riskli Kapalı Devre Harcama (Closed-Loop Merchant Network):**
   * İşçi nakde çevirmeyi beklemeden anlaşmalı esnafta (yemek, büfe, market) anında harcama yapabilir. Para zincirde kilitli olduğu için mağaza işverenin iflas riskinden etkilenmez, sıfır takas riski ve sıfır kredi kartı komisyonu ile anında tahsilat sağlar.
4. **Kod Seviyesinde İtiraz & Otomatik Borç Mahsubu (Automated Debt Netting):**
   * İş düzgün yapılmadıysa işveren itiraz eder. Bakiye önceden harcanmışsa işçinin hesabına otomatik borç (`debt`) yazılır. Ertesi günkü check-out hakedişinden **insan inisiyatifi olmadan** otomatik kesilir.

---

## 2. Soroban Akıllı Sözleşme Mimarisi (`contracts/stream_rail`)

### A. Depolama & TTL Stratejisi (State Archival Management)
Soroban State Archival standartlarına tam uyum:
* **`Instance Storage`:** Singleton yapılandırma verileri (`Admin`, `Token`, `AuthorityPubkey`, `MinShiftDuration`).
  * TTL: Her çağrıda `extend_ttl(100_000, 518_400)` (~30 günlük otomatik ömür uzatımı).
* **`Persistent Storage`:** İşveren teminatı, işçi bakiyesi/borcu, vardiyalar, mağaza alacakları ve kullanılmış QR nonce'ları.
  * Replay Attack Koruması: `DataKey::UsedNonce(shift_id)` anahtarı ile aynı QR kodunun mükerrer kullanımı engellenir.

### B. Tip Güvenli Hata Yönetimi (`#[contracterror]`)
```rust
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InsufficientBudget = 4,
    InsufficientBalance = 5,
    WageNotSet = 6,
    ShiftAlreadyActive = 7,
    NoActiveShift = 8,
    MinDurationNotMet = 9,
    InvalidSignature = 10,  // Sahte QR kodu tespiti
    QrExpired = 11,         // Süresi geçmiş QR kodu
    DisputeAmountTooHigh = 12,
}
```

### C. Çekirdek Fonksiyonlar
| Fonksiyon | İmza | Açıklama |
| :--- | :--- | :--- |
| `initialize` | `(admin, token, authority_pubkey, min_duration)` | Sözleşmeyi kurar, fintek token'ını ve amir imza anahtarını bağlar. |
| `lock_budget` | `(employer, amount)` | Faktoring/Kredi limitini teminat olarak sözleşmeye kilitler. |
| `set_wage` | `(employer, worker, daily_wage)` | Kişi bazlı günlük hak ediş ücreti tanımlar. |
| `check_in_with_qr`| `(worker, shift_id, expires_at, signature)` | Ed25519 imzalı, süreli ve replay-korumalı QR ile vardiya başlatır. |
| `check_out` | `(employer, worker)` | Süre kontrolü yapar, kilitli bütçeden düşer, varsa borcu mahsup eder. |
| `spend_at_merchant`| `(worker, merchant, amount)` | Hak edişi nakde çevirmeden anlaşmalı mağazaya aktarır. |
| `withdraw` | `(worker, amount)` | SEP-24 Anchor üzerinden TL olarak banka hesabına çekim yapar. |
| `merchant_withdraw`| `(merchant, amount)` | Mağazanın biriken alacaklarını bankaya çekmesini sağlar. |
| `dispute_checkout`| `(employer, worker, amount)` | İtiraz edilen tutarı işverene iade eder, işçiye borç yansıtır. |

---

## 3. Test & Doğrulama Kanıtı (Test Suite Verification)

Sözleşmenin tüm yaşam döngüsü `contracts/stream_rail/src/test.rs` altında uçtan uca simüle edilmiş ve **hatasız geçmiştir (`ok. 1 passed; 0 failed`)**:
1. Ed25519 anahtar çifti üretilerek gerçek kriptografik imza üretildi.
2. Sahte ve mükerrer QR check-in denemeleri kontrat tarafından reddedildi (`Replay attack error`).
3. Kredi limiti kilitlendi, check-in ve check-out tamamlandı.
4. Mağaza harcaması ve kalan tutarın nakit çekimi doğrulandı.
5. İtiraz senaryosunda işçiye 1.000 TL borç yazıldı; ertesi günkü vardiyada hakedişten otomatik kesilerek borç kapatıldı.

---

## 4. Dış Sistem Entegrasyonları (Backend & API Mimarisi)

* **Fintek / Faktoring Katmanı:** Banka/faktoring kurumunun muhasebe yazılımı, onaylanan kredi limiti için Stellar üzerinde SEP-41 uyumlu token mint eder veya transfer yetkisi verir.
* **Saha Amiri Mobil Uygulaması (QR Generator):** Saha yöneticisinin cihazındaki Ed25519 private key ile `sha256(shift_id + expires_at)` imzalanır ve ekranda 30 saniyelik dinamik QR gösterilir.
* **Olay Dinleyicisi (Indexer / Webhook):** Kontratın yaydığı `init`, `budget`, `check_in`, `chk_out`, `spend`, `withdraw`, `dispute` event'leri Mercury veya Zephyr aracılığıyla işverenin kurumsal ERP/muhasebe sistemine anlık işlenir.
