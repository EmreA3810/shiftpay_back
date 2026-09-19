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
fn test_shiftpay_accounting_and_real_transfer_lifecycle() {
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

    // 1. İşveren 20,000 token kilitler (kontrat havuzuna aktarılır)
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

    // 2. set_wage: Günlük ücret 1,000 TL
    client.set_wage(&employer, &worker, &1_000);
    assert_eq!(client.get_worker_wage(&worker), 1_000);

    // 3. check_in: İşçi işe başlar
    let shift_id: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
    client.check_in(&worker, &shift_id);
    assert!(client.get_shift(&worker).is_active);

    // Mesai süresi ilerletilir
    env.ledger().with_mut(|li| {
        li.timestamp += 10;
    });

    // -------------------------------------------------------------------------
    // TEST SENARYOSU 1:
    // İşçi check-out yapar → bakiyesi artar → hiçbir gerçek transfer/anchor işlemi tetiklenmemiş olmalı
    // -------------------------------------------------------------------------
    let new_balance = client.check_out(&worker);
    assert_eq!(new_balance, 1_000);
    assert_eq!(client.get_worker_balance(&worker), 1_000);
    assert!(!client.get_shift(&worker).is_active);

    // ÖNEMLİ KONTROL: Kontratın token bakiyesinden hiçbir şey düşmemiş olmalı (20,000 aynen duruyor)
    assert_eq!(token_client.balance(&contract_id), 20_000);
    // İşçiye veya mağazaya gerçek para gitmemiş olmalı
    assert_eq!(token_client.balance(&worker), 0);
    assert_eq!(token_client.balance(&merchant), 0);

    // -------------------------------------------------------------------------
    // TEST SENARYOSU 2:
    // İşçi spend_at_merchant çağırır → bakiyesi düşer, mağazaya gerçek transfer gider
    // -------------------------------------------------------------------------
    client.spend_at_merchant(&worker, &merchant, &400);
    // İşçinin muhasebe bakiyesi düşer: 1000 - 400 = 600
    assert_eq!(client.get_worker_balance(&worker), 600);
    // Mağazaya GERÇEK transfer gider: Mağaza bakiyesi 400 olur
    assert_eq!(token_client.balance(&merchant), 400);
    // Kontratın gerçek havuzundan 400 düşer: 20000 - 400 = 19600
    assert_eq!(token_client.balance(&contract_id), 19_600);

    // -------------------------------------------------------------------------
    // TEST SENARYOSU 3:
    // İşçi withdraw çağırır → bakiyesi düşer, anchor withdraw akışı başlar
    // -------------------------------------------------------------------------
    client.withdraw(&worker, &300);
    // İşçinin muhasebe bakiyesi düşer: 600 - 300 = 300
    assert_eq!(client.get_worker_balance(&worker), 300);
    // İşçiye / Anchor cüzdanına GERÇEK transfer gider: 300
    assert_eq!(token_client.balance(&worker), 300);
    // Kontratın gerçek havuzundan 300 daha düşer: 19600 - 300 = 19300
    assert_eq!(token_client.balance(&contract_id), 19_300);

    // -------------------------------------------------------------------------
    // TEST SENARYOSU 4:
    // İşçi bakiyesinden fazlasını harcamaya/çekmeye çalışırsa işlem reddedilmeli
    // -------------------------------------------------------------------------
    // Kalan bakiye: 300. 500 harcamaya çalışırsa Err(InsufficientClaimBalance)
    let spend_res = client.try_spend_at_merchant(&worker, &merchant, &500);
    assert!(spend_res.is_err());

    // 500 çekmeye çalışırsa Err(InsufficientClaimBalance)
    let withdraw_res = client.try_withdraw(&worker, &500);
    assert!(withdraw_res.is_err());

    // Bakiye ve token tutarları değişmemiş olmalı
    assert_eq!(client.get_worker_balance(&worker), 300);
    assert_eq!(token_client.balance(&worker), 300);
    assert_eq!(token_client.balance(&merchant), 400);
    assert_eq!(token_client.balance(&contract_id), 19_300);
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
