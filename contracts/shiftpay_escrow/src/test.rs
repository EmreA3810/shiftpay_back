// SPDX-License-Identifier: Apache-2.0
#![cfg(test)]
use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, BytesN, Env,
};

#[contract]
pub struct MockDeFindexVault;

#[contractimpl]
impl DeFindexVaultInterface for MockDeFindexVault {
    fn deposit(_env: Env, _from: Address, amount: i128) -> i128 {
        amount
    }
    fn withdraw(_env: Env, _to: Address, df_tokens: i128) -> i128 {
        df_tokens
    }
}

#[test]
fn test_shiftpay_daily_limit_and_30_day_maturity_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let supervisor = Address::generate(&env);
    let employer = Address::generate(&env);
    let worker = Address::generate(&env);
    let merchant = Address::generate(&env);

    let defindex_vault_id = env.register(MockDeFindexVault, ());

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = TokenClient::new(&env, &token_contract.address());
    let stellar_asset_client = StellarAssetClient::new(&env, &token_contract.address());

    stellar_asset_client.mint(&employer, &50_000);
    stellar_asset_client.mint(&admin, &10_000);

    let contract_id = env.register(ShiftPayEscrow, ());
    let client = ShiftPayEscrowClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &supervisor,
        &token_contract.address(),
        &Some(defindex_vault_id.clone()),
        &5, // min_duration: 5 seconds
    );

    // 1. İşveren 20,000 token kilitler
    client.deposit(
        &employer,
        &token_contract.address(),
        &20_000,
        &THIRTY_DAYS_SECONDS,
        &true,
    );
    assert_eq!(token_client.balance(&contract_id), 20_000);
    assert_eq!(token_client.balance(&worker), 0);
    assert_eq!(token_client.balance(&merchant), 0);

    // 2. İşverenin işçiye günlük ücreti tanımlaması: 1,000 TL
    client.set_wage(&employer, &worker, &1_000);

    // -------------------------------------------------------------------------
    // GÜN 1: Vardiya Tamamlandı, Limit Tanımlandı
    // -------------------------------------------------------------------------
    let shift_id_1: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
    client.check_in(&worker, &shift_id_1);
    env.ledger().with_mut(|li| { li.timestamp += 10; });

    let bal_day1 = client.check_out(&worker);
    assert_eq!(bal_day1, 1_000);
    assert_eq!(client.get_worker_balance(&worker), 1_000);

    // Kontratın kilitli havuzundan HİÇBİR ŞEY düşmedi (DeFindex getirisinde kalır)
    assert_eq!(token_client.balance(&contract_id), 20_000);

    // -------------------------------------------------------------------------
    // GÜN 1: İşçi Esnafta Hemen Harcama Yapabilir (Sıfır Vade)
    // -------------------------------------------------------------------------
    client.spend_at_merchant(&worker, &merchant, &400);
    assert_eq!(client.get_worker_balance(&worker), 600); // 1000 - 400 = 600
    assert_eq!(token_client.balance(&merchant), 400);    // Esnafa anında gitti
    assert_eq!(token_client.balance(&contract_id), 19_600);

    // -------------------------------------------------------------------------
    // GÜN 1: İşçi Henüz 30 Gün Dolmadan Nakit Çekim Yapamaz!
    // -------------------------------------------------------------------------
    let early_withdraw_res = client.try_withdraw(&worker, &200);
    assert!(early_withdraw_res.is_err()); // ClaimMaturityNotReached

    // -------------------------------------------------------------------------
    // GÜN 2: İşçi Yeni Bir Gün Çalışır, Limit Tekrar Eklenir (Birikir)
    // -------------------------------------------------------------------------
    let shift_id_2: BytesN<32> = BytesN::from_array(&env, &[2u8; 32]);
    client.check_in(&worker, &shift_id_2);
    env.ledger().with_mut(|li| { li.timestamp += 10; });

    let bal_day2 = client.check_out(&worker);
    // Kalan 600 TL limitine yeni günün 1,000 TL hak edişi eklendi = 1,600 TL
    assert_eq!(bal_day2, 1_600);
    assert_eq!(client.get_worker_balance(&worker), 1_600);

    // -------------------------------------------------------------------------
    // 30 GÜN SONRA: Vade Doldu, Mobilden Nakit Çekim Artık Yapılabilir
    // -------------------------------------------------------------------------
    env.ledger().with_mut(|li| {
        li.timestamp += THIRTY_DAYS_SECONDS + 1;
    });

    // 30 gün dolduğu için işçi 600 TL nakit çekim yapabilir
    client.withdraw(&worker, &600);
    assert_eq!(client.get_worker_balance(&worker), 1_000); // 1600 - 600 = 1000
    assert_eq!(token_client.balance(&worker), 600);       // İşçiye nakit aktarıldı
    assert_eq!(token_client.balance(&contract_id), 19_000);

    // Limitinden fazla çekmeye çalışırsa (1,000 varken 1,500 çekerse) reddedilir
    let overdraft_res = client.try_withdraw(&worker, &1_500);
    assert!(overdraft_res.is_err()); // InsufficientClaimBalance
}

#[test]
fn test_shiftpay_min_duration_and_double_checkout_protection() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let supervisor = Address::generate(&env);
    let employer = Address::generate(&env);
    let worker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let stellar_asset_client = StellarAssetClient::new(&env, &token_contract.address());
    stellar_asset_client.mint(&employer, &50_000);

    let contract_id = env.register(ShiftPayEscrow, ());
    let client = ShiftPayEscrowClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &supervisor,
        &token_contract.address(),
        &None,
        &60, // min_duration: 60 seconds
    );

    client.deposit(&employer, &token_contract.address(), &10_000, &THIRTY_DAYS_SECONDS, &false);
    client.set_wage(&employer, &worker, &1_200);

    // Vardiyasız check-out çağrısı hata vermeli
    assert!(client.try_check_out(&worker).is_err());

    // Vardiya başlat
    let shift_id: BytesN<32> = BytesN::from_array(&env, &[2u8; 32]);
    client.check_in(&worker, &shift_id);

    // 20 saniye sonra (60 saniye dolmadan) erken çıkış denenirse reddedilmeli
    env.ledger().with_mut(|li| { li.timestamp += 20; });
    assert!(client.try_check_out(&worker).is_err());

    // 60 saniye tamamlanınca çıkış başarılı olmalı
    env.ledger().with_mut(|li| { li.timestamp += 45; });
    let bal = client.check_out(&worker);
    assert_eq!(bal, 1_200);

    // İkinci defa çıkış denenirse hata vermeli
    assert!(client.try_check_out(&worker).is_err());
}
