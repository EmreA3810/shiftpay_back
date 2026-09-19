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
fn test_shiftpay_escrow_exact_spec_lifecycle() {
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

    // 1. deposit: Employer deposits 20,000 budget into DeFindex yield vault
    client.deposit(
        &employer,
        &token_contract.address(),
        &20_000,
        &THIRTY_DAYS_SECONDS,
        &true,
    );
    assert_eq!(client.get_employer_vault(&employer).locked_budget, 20_000);
    assert_eq!(client.get_employer_vault(&employer).defindex_shares, 20_000);

    // 2. set_wage: Employer sets daily wage to 1,000 TL
    client.set_wage(&employer, &worker, &1_000);
    assert_eq!(client.get_worker_wage(&worker), 1_000);

    // 3. check_in: Worker scans QR and checks in
    let shift_id: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
    client.check_in(&worker, &shift_id);
    assert!(client.get_shift(&worker).is_active);

    // Advance time
    env.ledger().with_mut(|li| {
        li.timestamp += 10;
    });

    // 4. check_out: Shift completed, wage credited to claim
    let claim = client.check_out(&employer, &worker);
    assert_eq!(claim.conditional_amount, 1_000);
    assert!(!client.get_shift(&worker).is_active);

    // 5. spend_at_merchant: Worker spends 400 at merchant
    client.spend_at_merchant(&worker, &merchant, &shift_id, &400);
    assert_eq!(client.get_worker_claim(&shift_id).unwrap().conditional_amount, 600);
    assert_eq!(client.get_merchant_balance(&merchant), 400);

    // 6. withdraw: Advance 30 days and withdraw remaining 600
    env.ledger().with_mut(|li| {
        li.timestamp += THIRTY_DAYS_SECONDS + 1;
    });
    client.withdraw(&worker, &shift_id, &token_contract.address());
    assert_eq!(token_client.balance(&worker), 600);

    // 7. dispute_checkout: Employer disputes 400 spent amount
    client.dispute_checkout(&employer, &worker, &shift_id, &400);
    let debt = client.get_worker_debt_state(&worker);
    assert_eq!(debt.is_debtor, true);
    assert_eq!(debt.total_debt, 400);
}
