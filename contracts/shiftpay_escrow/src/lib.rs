// SPDX-License-Identifier: Apache-2.0
#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short,
    token, Address, BytesN, Env,
};

// -----------------------------------------------------------------------------
// TTL / Instance Sabitleri
// -----------------------------------------------------------------------------
pub const DAY_IN_LEDGERS: u32 = 17280;
pub const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub const PERSISTENT_BUMP_AMOUNT: u32 = 60 * DAY_IN_LEDGERS;
pub const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub const THIRTY_DAYS_SECONDS: u64 = 30 * 24 * 60 * 60; // 2_592_000 saniye

// -----------------------------------------------------------------------------
// Hata Kodları (Error Codes)
// -----------------------------------------------------------------------------
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InsufficientVaultBudget = 4,
    InsufficientClaimBalance = 5,
    ClaimAlreadyExists = 6,
    ClaimNotFound = 7,
    ClaimAlreadyClaimed = 8,
    ClaimMaturityNotReached = 9,
    WorkerIsDebtor = 10,
    InsufficientSafetyReserve = 11,
    VaultNotConfigured = 12,
    WageNotSet = 13,
    ShiftAlreadyActive = 14,
    NoActiveShift = 15,
    MinDurationNotMet = 16,
}

// -----------------------------------------------------------------------------
// DeFindex Vault Client Arayüzü (DeFi - Yield)
// -----------------------------------------------------------------------------
#[contractclient(name = "DeFindexVaultClient")]
pub trait DeFindexVaultInterface {
    fn deposit(env: Env, from: Address, amount: i128) -> i128;
    fn withdraw(env: Env, to: Address, df_tokens: i128) -> i128;
}

// -----------------------------------------------------------------------------
// State & Structs
// -----------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct EmployerVault {
    pub locked_budget: i128,
    pub token_address: Address, // Nakit (USDC) veya Anchor Kredi Varlığı (TRY_CREDIT - SEP-41)
    pub employer: Address,
    pub lock_duration: u64,
    pub defindex_shares: i128,  // DeFindex Vault getiri payı
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Shift {
    pub start_time: u64,
    pub is_active: bool,
    pub shift_id: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WorkerClaim {
    pub worker: Address,
    pub shift_id: BytesN<32>,
    pub conditional_amount: i128,
    pub maturity_timestamp: u64,
    pub is_claimed: bool,
    pub is_transferred_to_merchant: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorkerDebtState {
    pub is_debtor: bool, // STATUS_DEBTOR bayrağı
    pub total_debt: i128,
    pub debt_repaid: i128,
    pub reputation_score: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Supervisor,
    DeFindexVault,
    SafetyReserveToken,
    SafetyReservePool,
    MinShiftDuration,
    TokenAddress,
    EmployerVault(Address),
    WorkerWage(Address),
    WorkerShift(Address),
    WorkerClaim(BytesN<32>),
    WorkerDebt(Address),
    MerchantBalance(Address),
    WorkerBalance(Address), // balances: Map<Address, i128> - her gün biriken harcama limiti
    WorkerWithdrawMaturity(Address), // 30 gün sonra nakit çekim vadesi (timestamp)
}

// -----------------------------------------------------------------------------
// Contract Implementation
// -----------------------------------------------------------------------------

#[contract]
pub struct ShiftPayEscrow;

#[contractimpl]
impl ShiftPayEscrow {
    /// Sözleşmeyi ilklendirir
    pub fn initialize(
        env: Env,
        admin: Address,
        supervisor: Address,
        reserve_token: Address,
        defindex_vault: Option<Address>,
        min_duration: u64,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Supervisor, &supervisor);
        env.storage().instance().set(&DataKey::SafetyReserveToken, &reserve_token);
        env.storage().instance().set(&DataKey::TokenAddress, &reserve_token);
        env.storage().instance().set(&DataKey::SafetyReservePool, &0i128);
        env.storage().instance().set(&DataKey::MinShiftDuration, &min_duration);

        if let Some(vault) = defindex_vault {
            env.storage().instance().set(&DataKey::DeFindexVault, &vault);
        }

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        Ok(())
    }

    /// Güvenlik Rezervi Fonlama
    pub fn fund_safety_reserve(env: Env, funder: Address, amount: i128) -> Result<(), Error> {
        funder.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let reserve_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::SafetyReserveToken)
            .ok_or(Error::NotInitialized)?;

        let client = token::Client::new(&env, &reserve_token);
        client.transfer(&funder, &env.current_contract_address(), &amount);

        let current_pool: i128 = env.storage().instance().get(&DataKey::SafetyReservePool).unwrap_or(0);
        let new_pool = current_pool + amount;
        env.storage().instance().set(&DataKey::SafetyReservePool, &new_pool);
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.events().publish((symbol_short!("reserve"), funder), (amount, new_pool));
        Ok(())
    }

    /// 1. deposit / lock_budget:
    /// İşveren sıcak nakit veya Anchor TRY_CREDIT limitini kilitler.
    /// Opsiyonel DeFindex Vault getiri yatırımı (auto_stake_defindex).
    pub fn deposit(
        env: Env,
        employer: Address,
        token_address: Address,
        amount: i128,
        lock_period_seconds: u64,
        auto_stake_defindex: bool,
    ) -> Result<(), Error> {
        employer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let client = token::Client::new(&env, &token_address);
        client.transfer(&employer, &env.current_contract_address(), &amount);

        env.storage().instance().set(&DataKey::TokenAddress, &token_address);

        let mut defindex_shares = 0i128;

        // DeFindex Vault Entegrasyonu
        if auto_stake_defindex {
            if let Some(vault_address) = env.storage().instance().get::<DataKey, Address>(&DataKey::DeFindexVault) {
                client.approve(&env.current_contract_address(), &vault_address, &amount, &100_000);
                let vault_client = DeFindexVaultClient::new(&env, &vault_address);
                defindex_shares = vault_client.deposit(&env.current_contract_address(), &amount);
            }
        }

        let vault_key = DataKey::EmployerVault(employer.clone());
        let mut vault: EmployerVault = env.storage().persistent().get(&vault_key).unwrap_or(EmployerVault {
            locked_budget: 0,
            token_address: token_address.clone(),
            employer: employer.clone(),
            lock_duration: lock_period_seconds,
            defindex_shares: 0,
        });

        vault.locked_budget += amount;
        vault.lock_duration = lock_period_seconds;
        vault.token_address = token_address;
        vault.defindex_shares += defindex_shares;

        env.storage().persistent().set(&vault_key, &vault);
        env.storage().persistent().extend_ttl(&vault_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        env.events().publish(
            (symbol_short!("deposit"), employer),
            (amount, vault.locked_budget, defindex_shares),
        );
        Ok(())
    }

    /// lock_budget takma adı (alias)
    pub fn lock_budget(
        env: Env,
        employer: Address,
        token_address: Address,
        amount: i128,
        lock_period_seconds: u64,
        auto_stake_defindex: bool,
    ) -> Result<(), Error> {
        Self::deposit(env, employer, token_address, amount, lock_period_seconds, auto_stake_defindex)
    }

    /// 2. set_wage: İşveren, çalışan bazında günlük ücreti tanımlar
    pub fn set_wage(env: Env, employer: Address, worker: Address, daily_wage: i128) -> Result<(), Error> {
        employer.require_auth();
        if daily_wage <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::WorkerWage(worker.clone());
        env.storage().persistent().set(&key, &daily_wage);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        env.events().publish((symbol_short!("set_wage"), employer, worker), daily_wage);
        Ok(())
    }

    /// 3. check_in: İşçi işe geldiğinde QR okutarak vardiyayı başlatır
    pub fn check_in(env: Env, worker: Address, shift_id: BytesN<32>) -> Result<(), Error> {
        worker.require_auth();

        let wage_key = DataKey::WorkerWage(worker.clone());
        if !env.storage().persistent().has(&wage_key) {
            return Err(Error::WageNotSet);
        }

        let shift_key = DataKey::WorkerShift(worker.clone());
        let shift: Shift = env.storage().persistent().get(&shift_key).unwrap_or(Shift {
            start_time: 0,
            is_active: false,
            shift_id: shift_id.clone(),
        });

        if shift.is_active {
            return Err(Error::ShiftAlreadyActive);
        }

        let now = env.ledger().timestamp();
        let new_shift = Shift {
            start_time: now,
            is_active: true,
            shift_id: shift_id.clone(),
        };
        env.storage().persistent().set(&shift_key, &new_shift);
        env.storage().persistent().extend_ttl(&shift_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        env.events().publish((symbol_short!("check_in"), worker), (shift_id, now));
        Ok(())
    }

    /// 4. check_out:
    /// Hiçbir transfer/ödeme işlemi yapmaz. Kontratın gerçek token bakiyesinden (pool_balance)
    /// hiçbir şey düşmez - sadece muhasebe kaydı (accounting) güncellenir.
    /// Sadece balances[işçi_adresi] += wages[işçi_adresi] şeklinde bakiyeyi artırır.
    /// Fonksiyon güncel bakiyeyi (i128) döner.
    pub fn check_out(env: Env, worker: Address) -> Result<i128, Error> {
        worker.require_auth();

        let shift_key = DataKey::WorkerShift(worker.clone());
        let shift: Shift = env
            .storage()
            .persistent()
            .get(&shift_key)
            .ok_or(Error::NoActiveShift)?;

        if !shift.is_active {
            return Err(Error::NoActiveShift);
        }

        let min_duration: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MinShiftDuration)
            .unwrap_or(0);

        let now = env.ledger().timestamp();
        let elapsed = now - shift.start_time;
        if elapsed < min_duration {
            return Err(Error::MinDurationNotMet);
        }

        let daily_wage: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::WorkerWage(worker.clone()))
            .ok_or(Error::WageNotSet)?;

        // Varsa temerrüt borcunu düş
        let net_earned = Self::auto_repay_debt(env.clone(), worker.clone(), daily_wage)?;

        // Sadece balances[işçi_adresi] += wages[işçi_adresi] - her gün limit tanımlanır ve birikir
        let balance_key = DataKey::WorkerBalance(worker.clone());
        let current_bal: i128 = env.storage().persistent().get(&balance_key).unwrap_or(0);
        let new_bal = current_bal + net_earned;

        env.storage().persistent().set(&balance_key, &new_bal);
        env.storage().persistent().extend_ttl(&balance_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        // 30 gün sonra nakit çekim vadesi (ilk vardiyada başlatılır)
        let maturity_key = DataKey::WorkerWithdrawMaturity(worker.clone());
        if !env.storage().persistent().has(&maturity_key) {
            let maturity_time = now + THIRTY_DAYS_SECONDS;
            env.storage().persistent().set(&maturity_key, &maturity_time);
            env.storage().persistent().extend_ttl(&maturity_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        }

        // Vardiyayı kapat
        env.storage().persistent().set(
            &shift_key,
            &Shift {
                start_time: 0,
                is_active: false,
                shift_id: shift.shift_id.clone(),
            },
        );

        env.events().publish(
            (symbol_short!("chk_out"), worker),
            (daily_wage, new_bal),
        );

        Ok(new_bal)
    }

    /// 5. spend_at_merchant:
    /// İşçi bakiyesinden anlaşmalı mağazada harcama yapar.
    /// require(tutar <= balances[işçi_adresi]) kontrolü yapılır, bakiyeden düşülür,
    /// mağazaya gerçek token transferi yapılır.
    pub fn spend_at_merchant(
        env: Env,
        worker: Address,
        merchant: Address,
        amount: i128,
    ) -> Result<(), Error> {
        worker.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let bal_key = DataKey::WorkerBalance(worker.clone());
        let current_bal: i128 = env.storage().persistent().get(&bal_key).unwrap_or(0);

        if current_bal < amount {
            return Err(Error::InsufficientClaimBalance);
        }

        let new_bal = current_bal - amount;
        env.storage().persistent().set(&bal_key, &new_bal);
        env.storage().persistent().extend_ttl(&bal_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        // Mağazaya gerçek transfer
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .or_else(|| env.storage().instance().get(&DataKey::SafetyReserveToken))
            .ok_or(Error::NotInitialized)?;

        let client = token::Client::new(&env, &token_address);
        client.transfer(&env.current_contract_address(), &merchant, &amount);

        // Esnaf alacak muhasebe kaydı
        let merchant_key = DataKey::MerchantBalance(merchant.clone());
        let current_m_bal: i128 = env.storage().persistent().get(&merchant_key).unwrap_or(0);
        let new_m_bal = current_m_bal + amount;
        env.storage().persistent().set(&merchant_key, &new_m_bal);
        env.storage().persistent().extend_ttl(&merchant_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        env.events().publish(
            (symbol_short!("spend"), worker, merchant),
            (amount, new_bal),
        );

        Ok(())
    }

    /// 6. withdraw:
    /// İşçi kullanılabilir bakiyesini Anchor (SEP-24) veya cüzdanına çeker.
    /// require(tutar <= balances[işçi_adresi]) kontrolü yapılır, bakiyeden düşülür,
    /// anchor SEP-24 / gerçek transfer akışını tetikler.
    pub fn withdraw(
        env: Env,
        worker: Address,
        amount: i128,
    ) -> Result<(), Error> {
        worker.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // 30 gün vade kontrolü (Nakit çekim sadece vade dolunca yapılabilir)
        let maturity_key = DataKey::WorkerWithdrawMaturity(worker.clone());
        let maturity_time: u64 = env.storage().persistent().get(&maturity_key).unwrap_or(0);
        let now = env.ledger().timestamp();
        if now < maturity_time {
            return Err(Error::ClaimMaturityNotReached);
        }

        let bal_key = DataKey::WorkerBalance(worker.clone());
        let current_bal: i128 = env.storage().persistent().get(&bal_key).unwrap_or(0);

        if current_bal < amount {
            return Err(Error::InsufficientClaimBalance);
        }

        let new_bal = current_bal - amount;
        env.storage().persistent().set(&bal_key, &new_bal);
        env.storage().persistent().extend_ttl(&bal_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .or_else(|| env.storage().instance().get(&DataKey::SafetyReserveToken))
            .ok_or(Error::NotInitialized)?;

        let client = token::Client::new(&env, &token_address);
        client.transfer(&env.current_contract_address(), &worker, &amount);

        env.events().publish((symbol_short!("withdraw"), worker), (amount, new_bal));
        Ok(())
    }

    /// 7. dispute_checkout: İhlal durumunda işçi temerrüde düşürülür
    pub fn dispute_checkout(
        env: Env,
        employer: Address,
        worker: Address,
        spent_amount: i128,
    ) -> Result<(), Error> {
        employer.require_auth();

        if spent_amount < 0 {
            return Err(Error::InvalidAmount);
        }

        let debt_key = DataKey::WorkerDebt(worker.clone());
        let mut debt_state: WorkerDebtState = env.storage().persistent().get(&debt_key).unwrap_or(WorkerDebtState {
            is_debtor: false,
            total_debt: 0,
            debt_repaid: 0,
            reputation_score: 100,
        });

        debt_state.is_debtor = true;
        debt_state.total_debt += spent_amount;
        debt_state.reputation_score = 0;

        env.storage().persistent().set(&debt_key, &debt_state);
        env.storage().persistent().extend_ttl(&debt_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        // İşçinin kullanılabilir bakiyesini sıfırla
        let bal_key = DataKey::WorkerBalance(worker.clone());
        env.storage().persistent().set(&bal_key, &0i128);

        env.events().publish(
            (symbol_short!("dispute"), employer, worker),
            (spent_amount, debt_state.total_debt),
        );

        Ok(())
    }

    /// 8. auto_repay_debt: Temerrüde düşmüş işçinin yeni kazançlarından borcu otomatik tahsil eder.
    pub fn auto_repay_debt(env: Env, worker: Address, new_earnings: i128) -> Result<i128, Error> {
        let debt_key = DataKey::WorkerDebt(worker.clone());
        let mut debt_state: WorkerDebtState = env.storage().persistent().get(&debt_key).unwrap_or_default();

        if !debt_state.is_debtor || debt_state.total_debt == 0 {
            return Ok(new_earnings);
        }

        let mut remaining_earnings = new_earnings;
        let repaid_now: i128;

        if remaining_earnings >= debt_state.total_debt {
            repaid_now = debt_state.total_debt;
            remaining_earnings -= debt_state.total_debt;
            debt_state.debt_repaid += debt_state.total_debt;
            debt_state.total_debt = 0;
            debt_state.is_debtor = false;
            debt_state.reputation_score = 50;
        } else {
            repaid_now = remaining_earnings;
            debt_state.total_debt -= remaining_earnings;
            debt_state.debt_repaid += remaining_earnings;
            remaining_earnings = 0;
        }

        env.storage().persistent().set(&debt_key, &debt_state);
        env.storage().persistent().extend_ttl(&debt_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        env.events().publish(
            (symbol_short!("debt_pay"), worker),
            (repaid_now, debt_state.total_debt, remaining_earnings),
        );

        Ok(remaining_earnings)
    }

    /// 9. merchant_withdraw: Esnaf biriken hak edişini çeker
    pub fn merchant_withdraw(
        env: Env,
        merchant: Address,
        token_address: Address,
        amount: i128,
    ) -> Result<(), Error> {
        merchant.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let merchant_key = DataKey::MerchantBalance(merchant.clone());
        let mut m_bal: i128 = env.storage().persistent().get(&merchant_key).unwrap_or(0);

        if m_bal < amount {
            return Err(Error::InsufficientClaimBalance);
        }

        m_bal -= amount;
        env.storage().persistent().set(&merchant_key, &m_bal);
        env.storage().persistent().extend_ttl(&merchant_key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        let client = token::Client::new(&env, &token_address);
        client.transfer(&env.current_contract_address(), &merchant, &amount);

        env.events().publish((symbol_short!("m_withdr"), merchant), amount);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Getter Fonksiyonları
    // -------------------------------------------------------------------------

    pub fn get_worker_balance(env: Env, worker: Address) -> i128 {
        env.storage().persistent().get(&DataKey::WorkerBalance(worker)).unwrap_or(0)
    }

    pub fn get_worker_withdraw_maturity(env: Env, worker: Address) -> u64 {
        env.storage().persistent().get(&DataKey::WorkerWithdrawMaturity(worker)).unwrap_or(0)
    }

    pub fn get_employer_vault(env: Env, employer: Address) -> EmployerVault {
        env.storage().persistent().get(&DataKey::EmployerVault(employer.clone())).unwrap_or(EmployerVault {
            locked_budget: 0,
            token_address: employer.clone(),
            employer,
            lock_duration: 0,
            defindex_shares: 0,
        })
    }

    pub fn get_worker_wage(env: Env, worker: Address) -> i128 {
        env.storage().persistent().get(&DataKey::WorkerWage(worker)).unwrap_or(0)
    }

    pub fn get_shift(env: Env, worker: Address) -> Shift {
        env.storage().persistent().get(&DataKey::WorkerShift(worker)).unwrap_or(Shift {
            start_time: 0,
            is_active: false,
            shift_id: BytesN::from_array(&env, &[0u8; 32]),
        })
    }

    pub fn get_worker_debt_state(env: Env, worker: Address) -> WorkerDebtState {
        env.storage().persistent().get(&DataKey::WorkerDebt(worker)).unwrap_or_default()
    }

    pub fn get_merchant_balance(env: Env, merchant: Address) -> i128 {
        env.storage().persistent().get(&DataKey::MerchantBalance(merchant)).unwrap_or(0)
    }

    pub fn get_safety_reserve_pool(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::SafetyReservePool).unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
