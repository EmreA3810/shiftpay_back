# ShiftPay Escrow — Pro Hackathon 2026 Proje ve Mimari Dokümanı
**Track:** Genesis Track  
**Venue:** Grand Pera, Beyoğlu, Istanbul | Rise In x Stellar  

---

## 📌 Resmi Stellar Skill & Protokol Atıfları (Official Skill & Partner Citations)
Hackathon kurallarına ve partner listesine uygun olarak projemizde şu bileşenler entegre edilmiş ve test edilmiştir:
1. `skills/smart-contracts/SKILL.md`: Rust Soroban akıllı sözleşme mimarisi, depolama (storage) ve yetkilendirme (auth) kılavuzu.
2. `skills/standards/SKILL.md`: SEP-41 (Token Interface), SEP-24 (Interactive Anchor), SEP-6 standartları.
3. `skills/dapp/SKILL.md`: Stellar Wallets Kit ve Passkey dApp entegrasyon kılavuzu.
4. `skills/assets/SKILL.md`: Kurumsal faktoring `TRY_CREDIT` varlığı ve SAC köprü yönetimi.
5. `skills/defindex/SKILL.md`: DeFindex Vault getiri (yield) SDK ve sözleşme entegrasyon rehberi.
6. **Privy (Wallets / Social Onboarding Partner):** Kılavuzdaki resmi partner listesinde yer alan Privy ile Google Login ve tohumsuz gömülü cüzdan (embedded wallet) yönetimi (`scripts/privy_auth.js`).
7. **Stellar Wallets Kit (Multi-Wallet Connector Partner):** İşçi, işveren ve esnaf için Freighter, LOBSTR, xBull, Albedo cüzdan bağlantı kiti (`scripts/wallets_kit_integration.js`).

---

## 1. Problem ve Çözüm (The Narrative “Why”)

### Problem
Etkinlik, fuar, anket ve inşaat gibi saha personeli ve freelancer sektörlerinde en büyük sorun **"hak edişin zamanında ödenmesi ve nakit akışı"**dır.
* **İşçi:** "Günlerce çalıştım, paramı alabilecek miyim?" endişesi yaşar.
* **İşveren/Ajans:** Ana müşterisinden (ilaç firması, organizatör) ödemeyi 30-60 gün vadeli alır; bu nedenle işçilere peşin sıcak nakit ödemekte zorlanır.
* **Geleneksel Bankacılık Eksikliği:** Banka sistemi güven ilişkisi kuramaz; esnaf işçinin "akşam maaşım yatacak" sözüne güvenip yemek/içecek veremez.

### Çözümümüz: ShiftPay Escrow
İşverenin banka/faktoring kurumundan aldığı on-chain kredi limitini (**SEP-41 `TRY_CREDIT`**) teminat kilitlediği, kilitli fonların **DeFindex Vault** havuzunda beklerken getiri ürettiği, işçi, işveren ve esnafın **Stellar Wallets Kit & Privy (Google)** ile cüzdansız veya yerel cüzdanlarla bağlandığı, işçinin çalıştığı an **30 gün vadeli şartlı hak ediş varlığı** kazandığı, bu hak edişi nakde çevirmeden anlaşmalı esnafta sıfır riskle harcayabildiği ve vadesinde **SEP-24 Anchor ile hem işçinin hem mağazanın doğrudan gerçek TL olarak banka hesabına (FAST) çekebildiği** bir ödeme protokolüdür.

---

## 2. Özel Soroban Sözleşmesi Çekirdek Fonksiyonları (Core Contract Functions)

[`contracts/shiftpay_escrow/src/lib.rs`](contracts/shiftpay_escrow/src/lib.rs) sözleşmesinde tam olarak uygulanan çekirdek fonksiyon seti:

| Fonksiyon Adı | Açıklama |
| :--- | :--- |
| **`deposit` / `lock_budget`** | İşveren bütçesini / faktoring limitini kilitler, opsiyonel olarak fonu otomatik DeFindex Vault'a yatırarak getiri üretir. |
| **`set_wage`** | İşveren çalışan bazında günlük hak ediş tutarını tanımlar. |
| **`check_in`** | İşçi işe geldiğinde QR okutarak vardiyayı ve blokzincir zaman damgasını başlatır. |
| **`check_out`** | Minimum vardiya süresini kontrol eder, hakedişi oluşturur ve varsa geçmiş borcu otomatik mahsup eder. |
| **`spend_at_merchant`** | İşçi hak edişini nakde çevirmeden anlaşmalı esnafta (kahve, yemek) anında harcar (sıfır takas riski). |
| **`withdraw`** | Vadesi gelen hak edişi SEP-24 Anchor üzerinden gerçek TL olarak banka hesabına aktarır. |
| **`dispute_checkout`** | İşveren itirazı durumunda işçiyi `STATUS_DEBTOR` ilan eder, itibarını sıfırlar ve Güvenlik Rezervini tetikler. |
| **`merchant_withdraw`** | Mağazanın kazandığı stabilcoin/alacakları SEP-24 Anchor üzerinden kendi TL banka hesabına çekmesini sağlar. |
| **`auto_repay_debt`** | Temerrüde düşmüş işçinin yeni kazançlarından eski borcu kod seviyesinde otomatik tahsil eder. |

---

## 3. Kılavuz Şartlarına Tam Uyum Matrisi (Compliance Matrix)

| Kılavuz Şartı | Partner / Standart | Nasıl Karşılandı? | Teknik Kanıt |
| :--- | :--- | :--- | :--- |
| **Anchor / Local Payments (Ağırlıklı)** | **SEP-24 / Testnet Anchor** | **Hem işçinin TL çekimi hem mağazanın aldığı stabilcoin'i kendi TL'sine çevirmesi (FAST 7/24)** | `scripts/anchor_integration.js` (`withdrawWorkerWage` & `withdrawMerchantRevenue`) |
| **Wallets Partner** | **Stellar Wallets Kit** | **İşçi, işveren ve mağaza cüzdan bağlantıları (Freighter, LOBSTR, Albedo, xBull)** | `scripts/wallets_kit_integration.js` |
| **Wallets Partner (Social)** | **Privy** | Google ile tek tıkla giriş ve tohumsuz gömülü cüzdan | `scripts/privy_auth.js` |
| **DeFi - Yield Partner** | **DeFindex** | Kilitli bütçenin DeFindex Vault'ta getiri üretmesi | `contracts/shiftpay_escrow/src/lib.rs` & `scripts/defindex_integration.js` |
| **Core Feature** | **Özel Soroban Sözleşmesi** | `deposit`, `set_wage`, `check_in`, `check_out`, `spend_at_merchant`, `withdraw`, `dispute_checkout` | `contracts/shiftpay_escrow/src/lib.rs` & `test.rs` |

---

## 4. Sistem Mimarisi (Architecture Diagram)

```mermaid
graph TD
    subgraph "1. Cüzdan & Giriş (Stellar Wallets Kit & Privy)"
        WorkerUser[👷 Saha İşçisi] -->|Albedo / LOBSTR / Google| WalletsKit[💼 Stellar Wallets Kit]
        EmployerUser[🏢 İşveren / Ajans] -->|Freighter Corporate| WalletsKit
        MerchantUser[☕ Mağaza / Esnaf POS] -->|LOBSTR POS / Albedo| WalletsKit
    end

    subgraph "2. Giriş & Teminat (DeFindex Entegrasyonu)"
        EmployerUser -->|1. TL Yatırır / Kredi Limiti| AnchorIn[🏦 SEP-24 TRY Anchor]
        AnchorIn -->|TRY_CREDIT / USDC| Vault[📦 EmployerVault / ShiftPayEscrow]
        Vault -->|2. Otomatik Getiri Sağla| DeFindex[📈 DeFindex Vault]
    end

    subgraph "3. Saha Vardiyası & Hakediş (Soroban)"
        Supervisor[🕵️ Saha Amiri / Oracle] -->|3. Vardiya Onayı| ShiftPay[⚡ ShiftPay Escrow Contract]
        ShiftPay -->|check_in & check_out| WorkerClaimData[👷 WorkerClaim]
    end

    subgraph "4. Harcama & Çift Yönlü Anchor Çıkışı (Local Rails)"
        WorkerClaimData -->|spend_at_merchant| MerchantStore[☕ Anlaşmalı Esnaf]
        WorkerClaimData -->|withdraw| AnchorWorker[🏛️ SEP-24 Anchor (İşçi TL IBAN)]
        MerchantStore -->|merchant_withdraw| AnchorMerchant[🏛️ SEP-24 Anchor (Mağaza Ticari TL IBAN)]
    end

    subgraph "5. İtiraz, Temerrüt & Otomatik Borç Kapatma"
        EmployerUser -.->|dispute_checkout| Default[🚨 handle_worker_default]
        Default -->|STATUS_DEBTOR & 0 İtibar| WorkerDebt[(WorkerDebtState)]
        Default -->|Zarar Karşıla| SafetyReserve[🛡️ Safety Reserve Pool]
        WorkerDebt -->|Sonraki Kazançtan Kes| AutoRepay[🔄 auto_repay_debt]
    end
```

---

## 5. Test ve Doğrulama Kanıtları (Test Verification)

1. **Soroban Akıllı Sözleşmesi (`test.rs`):**
   ```text
   running 1 test
   test test::test_shiftpay_escrow_exact_spec_lifecycle ... ok
   test result: ok. 1 passed; 0 failed; finished in 0.10s
   ```
2. **Anchor SEP-24 Çift Yönlü Çekim (`anchor_integration.js`):**
   * İşveren TL yatırma (`depositEmployerBudget`),
   * İşçi TL çekimi (`withdrawWorkerWage`),
   * Mağaza kazancını TL'ye çevirme (`withdrawMerchantRevenue`) hatasız çalıştırıldı.
3. **Stellar Wallets Kit Çoklu Rol Bağlantısı (`wallets_kit_integration.js`):**
   * İşçi (Albedo), İşveren (Freighter) ve Mağaza (LOBSTR POS) cüzdan bağlantıları ve işlem imzalamaları doğrulandı.
4. **DeFindex SDK Entegrasyonu (`defindex_integration.js`):**
   * Vault XDR yatırma ve çekme akışı başarıyla test edildi.
