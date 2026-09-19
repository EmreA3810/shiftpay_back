// StreamRail — Interactive Simulator connected with Soroban Contract Logic

// State
const state = {
  employerBudget: 5000,
  dailyWage: 500,
  workerEarned: 0,
  workerDebt: 0,
  merchantBalance: 0,
  shiftActive: false,
  shiftStartTime: null,
  minDurationSeconds: 5,
  timerInterval: null
};

// DOM Elements
const roleBtns = document.querySelectorAll('.role-btn');
const views = {
  worker: document.getElementById('view-worker'),
  employer: document.getElementById('view-employer'),
  merchant: document.getElementById('view-merchant')
};

// Summary Stats
const statEmployerBudget = document.getElementById('stat-employer-budget');
const statWorkerBalance = document.getElementById('stat-worker-balance');
const statDebtTag = document.getElementById('stat-debt-tag');
const statMerchantBalance = document.getElementById('stat-merchant-balance');

// Worker View Elements
const shiftStatusDot = document.getElementById('shift-status-dot');
const shiftStatusText = document.getElementById('shift-status-text');
const shiftTimerText = document.getElementById('shift-timer-text');
const scannerLaser = document.getElementById('scanner-laser');
const btnCheckin = document.getElementById('btn-checkin');
const btnCheckout = document.getElementById('btn-checkout');
const workerAvailableAmt = document.getElementById('worker-available-amt');
const debtAlert = document.getElementById('debt-alert');
const debtAmountText = document.getElementById('debt-amount-text');
const tabSpend = document.getElementById('tab-spend');
const tabWithdraw = document.getElementById('tab-withdraw');
const tabContentSpend = document.getElementById('tab-content-spend');
const tabContentWithdraw = document.getElementById('tab-content-withdraw');
const spendMerchantSelect = document.getElementById('spend-merchant-select');
const spendAmountInput = document.getElementById('spend-amount-input');
const btnSpendMerchant = document.getElementById('btn-spend-merchant');
const withdrawAmountInput = document.getElementById('withdraw-amount-input');
const btnWithdrawAnchor = document.getElementById('btn-withdraw-anchor');

// Employer View Elements
const budgetAmountInput = document.getElementById('budget-amount-input');
const btnLockBudget = document.getElementById('btn-lock-budget');
const dailyWageInput = document.getElementById('daily-wage-input');
const btnSetWage = document.getElementById('btn-set-wage');
const disputeAmountInput = document.getElementById('dispute-amount-input');
const btnDispute = document.getElementById('btn-dispute');
const eventLogs = document.getElementById('event-logs');

// Merchant View Elements
const merchantAvailableAmt = document.getElementById('merchant-available-amt');
const btnMerchantSettle = document.getElementById('btn-merchant-settle');

// Log Helper
function addLog(msg) {
  const time = new Date().toLocaleTimeString('tr-TR');
  const entry = document.createElement('div');
  entry.className = 'log-entry';
  entry.innerHTML = `<span class="log-time">[${time}]</span> ${msg}`;
  eventLogs.appendChild(entry);
  eventLogs.scrollTop = eventLogs.scrollHeight;
}

// Update UI
function updateUI() {
  statEmployerBudget.innerHTML = `₺${state.employerBudget.toLocaleString('tr-TR', {minimumFractionDigits: 2})} <small>TRY_CREDIT</small>`;
  statWorkerBalance.innerHTML = `₺${state.workerEarned.toLocaleString('tr-TR', {minimumFractionDigits: 2})}`;
  
  if (state.workerDebt > 0) {
    statDebtTag.style.display = 'inline-block';
    statDebtTag.textContent = `-₺${state.workerDebt} Borç`;
    debtAlert.style.display = 'block';
    debtAmountText.textContent = `₺${state.workerDebt.toLocaleString('tr-TR', {minimumFractionDigits: 2})}`;
  } else {
    statDebtTag.style.display = 'none';
    debtAlert.style.display = 'none';
  }

  statMerchantBalance.textContent = `₺${state.merchantBalance.toLocaleString('tr-TR', {minimumFractionDigits: 2})}`;
  workerAvailableAmt.textContent = `₺${state.workerEarned.toLocaleString('tr-TR', {minimumFractionDigits: 2})}`;
  merchantAvailableAmt.textContent = `₺${state.merchantBalance.toLocaleString('tr-TR', {minimumFractionDigits: 2})}`;

  // Shift UI
  if (state.shiftActive) {
    shiftStatusDot.className = 'status-indicator active';
    shiftStatusText.textContent = 'Vardiya Devam Ediyor';
    scannerLaser.style.display = 'block';
    btnCheckin.disabled = true;
  } else {
    shiftStatusDot.className = 'status-indicator';
    shiftStatusText.textContent = 'Mesai Başlamadı';
    shiftTimerText.textContent = 'Süre: 00:00:00 (Minimum: 5 sn)';
    scannerLaser.style.display = 'none';
    btnCheckin.disabled = false;
    btnCheckout.disabled = true;
  }
}

// Role Switching
roleBtns.forEach(btn => {
  btn.addEventListener('click', () => {
    roleBtns.forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    const role = btn.dataset.role;
    Object.keys(views).forEach(k => {
      views[k].classList.toggle('active', k === role);
    });
  });
});

// Worker Tabs
tabSpend.addEventListener('click', () => {
  tabSpend.classList.add('active');
  tabWithdraw.classList.remove('active');
  tabContentSpend.style.display = 'block';
  tabContentWithdraw.style.display = 'none';
});

tabWithdraw.addEventListener('click', () => {
  tabWithdraw.classList.add('active');
  tabSpend.classList.remove('active');
  tabContentWithdraw.style.display = 'block';
  tabContentSpend.style.display = 'none';
});

// 1. Worker Check-in
btnCheckin.addEventListener('click', () => {
  if (state.shiftActive) return;
  state.shiftActive = true;
  state.shiftStartTime = Date.now();
  addLog(`👷 İşçi QR okuttu: Check-in yapıldı. (Soroban: check_in)`);

  state.timerInterval = setInterval(() => {
    const elapsedSec = Math.floor((Date.now() - state.shiftStartTime) / 1000);
    const m = String(Math.floor(elapsedSec / 60)).padStart(2, '0');
    const s = String(elapsedSec % 60).padStart(2, '0');
    shiftTimerText.textContent = `Süre: 00:${m}:${s} ${elapsedSec >= state.minDurationSeconds ? '✅ Minimum süre doldu!' : `(Minimum: ${state.minDurationSeconds - elapsedSec} sn kaldı)`}`;

    if (elapsedSec >= state.minDurationSeconds) {
      btnCheckout.disabled = false;
    }
  }, 1000);

  updateUI();
});

// 2. Worker Check-out
btnCheckout.addEventListener('click', () => {
  if (!state.shiftActive) return;
  clearInterval(state.timerInterval);
  state.shiftActive = false;

  const wage = state.dailyWage;
  if (state.employerBudget < wage) {
    alert('İşverenin kilitli bütçesi yetersiz!');
    updateUI();
    return;
  }

  // Deduct from employer
  state.employerBudget -= wage;

  // Debt netting logic
  let netEarned = wage;
  let settledDebt = 0;
  if (state.workerDebt > 0) {
    if (netEarned >= state.workerDebt) {
      settledDebt = state.workerDebt;
      netEarned -= state.workerDebt;
      state.workerDebt = 0;
    } else {
      settledDebt = netEarned;
      state.workerDebt -= netEarned;
      netEarned = 0;
    }
  }

  state.workerEarned += netEarned;

  addLog(`🏁 Check-out tamamlandı! Hak ediş: ₺${wage}. ${settledDebt > 0 ? `(₺${settledDebt} geçmiş borca mahsup edildi).` : ''} Net eklenen: ₺${netEarned}. (Soroban: check_out)`);
  updateUI();
});

// 3. Spend at Merchant
btnSpendMerchant.addEventListener('click', () => {
  const amt = parseFloat(spendAmountInput.value) || 0;
  if (amt <= 0) return alert('Geçerli bir tutar girin');
  if (amt > state.workerEarned) return alert('Yetersiz hak ediş bakiyesi!');

  state.workerEarned -= amt;
  state.merchantBalance += amt;
  const merchantName = spendMerchantSelect.value;

  addLog(`🛍️ İşçi mağazada harcadı: ₺${amt} -> "${merchantName}". Kredi alacağı mağazaya aktarıldı. (Soroban: spend_at_merchant)`);
  updateUI();
});

// 4. Withdraw via Anchor
btnWithdrawAnchor.addEventListener('click', () => {
  const amt = parseFloat(withdrawAmountInput.value) || 0;
  if (amt <= 0) return alert('Geçerli bir tutar girin');
  if (amt > state.workerEarned) return alert('Yetersiz hak ediş bakiyesi!');

  state.workerEarned -= amt;
  addLog(`🏛️ SEP-24 Anchor çekim akışı açıldı: ₺${amt} banka hesabına gönderiliyor. (Soroban: withdraw)`);
  alert(`✅ SEP-24 Başarılı!\n₺${amt} tutarındaki hakedişiniz IBAN hesabınıza aktarıldı.`);
  updateUI();
});

// 5. Employer Lock Budget
btnLockBudget.addEventListener('click', () => {
  const amt = parseFloat(budgetAmountInput.value) || 0;
  if (amt <= 0) return alert('Geçerli bir tutar girin');

  state.employerBudget += amt;
  addLog(`🔒 İşveren fintek kredi limitini kilitledi: +₺${amt} TRY_CREDIT. (Soroban: lock_budget)`);
  updateUI();
});

// 6. Employer Set Wage
btnSetWage.addEventListener('click', () => {
  const wage = parseFloat(dailyWageInput.value) || 0;
  if (wage <= 0) return alert('Geçerli bir ücret girin');
  state.dailyWage = wage;
  addLog(`💾 İşçi için günlük hakediş güncellendi: ₺${wage}/gün. (Soroban: set_wage)`);
  alert('Günlük ücret güncellendi!');
  updateUI();
});

// 7. Employer Dispute
btnDispute.addEventListener('click', () => {
  const disputeAmt = parseFloat(disputeAmountInput.value) || 0;
  if (disputeAmt <= 0) return alert('Geçerli bir tutar girin');

  // If worker has balance, reduce it
  if (state.workerEarned >= disputeAmt) {
    state.workerEarned -= disputeAmt;
  } else {
    const deficit = disputeAmt - state.workerEarned;
    state.workerEarned = 0;
    state.workerDebt += deficit; // Negative balance / debt written!
  }

  state.employerBudget += disputeAmt; // returned to employer
  addLog(`🚨 İŞVEREN İTİRAZ ETTİ (Dispute): ₺${disputeAmt}. Bakiye yetersiz kaldığı için işçiye borç yazıldı. (Soroban: dispute_checkout)`);
  alert(`İtiraz işleme alındı! ₺${disputeAmt} işverene iade edildi.`);
  updateUI();
});

// 8. Merchant Settle
btnMerchantSettle.addEventListener('click', () => {
  if (state.merchantBalance <= 0) return alert('Çekilebilir mağaza alacağı yok!');
  const amt = state.merchantBalance;
  state.merchantBalance = 0;
  addLog(`🏛️ Mağaza alacaklarını tahsil etti: ₺${amt} banka hesabına çekildi. (Soroban: merchant_withdraw)`);
  alert(`✅ Mağaza Kapanışı: ₺${amt} banka hesabınıza aktarıldı.`);
  updateUI();
});

// Init
updateUI();
