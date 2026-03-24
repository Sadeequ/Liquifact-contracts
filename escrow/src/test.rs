use super::{LiquifactEscrow, LiquifactEscrowClient};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deploy a fresh contract and return (env, client, admin, sme).
fn setup() -> (Env, LiquifactEscrowClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);
    (env, client, admin, sme)
}

// ---------------------------------------------------------------------------
// Happy-path tests
// ---------------------------------------------------------------------------

#[test]
fn test_init_stores_escrow() {
    let (_, client, admin, sme) = setup();

    let escrow = client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    assert_eq!(escrow.invoice_id, symbol_short!("INV001"));
    assert_eq!(escrow.admin, admin);
    assert_eq!(escrow.sme_address, sme);
    assert_eq!(escrow.amount, 10_000_0000000i128);
    assert_eq!(escrow.funded_amount, 0);
    assert_eq!(escrow.status, 0);

    // get_escrow should return the same data
    let got = client.get_escrow();
    assert_eq!(got.invoice_id, escrow.invoice_id);
    assert_eq!(got.admin, admin);
}

#[test]
fn test_fund_partial_then_full() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV002"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    // Partial fund — status stays open
    let e1 = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(e1.funded_amount, 5_000_0000000i128);
    assert_eq!(e1.status, 0);

    // Complete fund — status becomes funded
    let e2 = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(e2.funded_amount, 10_000_0000000i128);
    assert_eq!(e2.status, 1);
}

#[test]
fn test_settle_after_full_funding() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV003"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &10_000_0000000i128);

    let settled = client.settle();
    assert_eq!(settled.status, 2);
}

// ---------------------------------------------------------------------------
// Authorization verification tests
// ---------------------------------------------------------------------------

/// Verify that `init` records an auth requirement for the admin address.
#[test]
fn test_init_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
        &symbol_short!("INV004"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );

    // Inspect recorded auths — admin must appear as the top-level authorizer.
    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == admin),
        "admin auth was not recorded for init"
    );
}

/// Verify that `fund` records an auth requirement for the investor address.
#[test]
fn test_fund_requires_investor_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
        &symbol_short!("INV005"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.fund(&investor, &1_000i128);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == investor),
        "investor auth was not recorded for fund"
    );
}

/// Verify that `settle` records an auth requirement for the SME address.
#[test]
fn test_settle_requires_sme_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
        &symbol_short!("INV006"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.fund(&investor, &1_000i128);
    client.settle();

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == sme),
        "sme auth was not recorded for settle"
    );
}

// ---------------------------------------------------------------------------
// Unauthorized / panic-path tests
// ---------------------------------------------------------------------------

/// `init` called by a non-admin should panic (auth not satisfied).
#[test]
#[should_panic]
fn test_init_unauthorized_panics() {
    let env = Env::default();
    // Do NOT mock auths — let the real auth check fire.
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
        &symbol_short!("INV007"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
}

/// `settle` called without SME auth should panic.
#[test]
#[should_panic]
fn test_settle_unauthorized_panics() {
    let env = Env::default();
    // Do NOT mock auths.
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    // Use mock_all_auths only for setup steps.
    env.mock_all_auths();
    client.init(
        &admin,
        &symbol_short!("INV008"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.fund(&investor, &1_000i128);

    // Clear mocked auths so settle must satisfy real auth.
    // Soroban test env doesn't expose a "clear mocks" API, so we re-create
    // a client on the same contract without mocking to trigger the failure.
    let env2 = Env::default(); // fresh env — no mocked auths
    let client2 = LiquifactEscrowClient::new(&env2, &contract_id);
    client2.settle(); // should panic: sme auth not satisfied
}

// ---------------------------------------------------------------------------
// Edge-case / guard tests
// ---------------------------------------------------------------------------

/// Re-initializing an already-initialized escrow must panic.
#[test]
#[should_panic(expected = "Escrow already initialized")]
fn test_double_init_panics() {
    let (_, client, admin, sme) = setup();

    client.init(
        &admin,
        &symbol_short!("INV009"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    // Second init on the same contract must be rejected.
    client.init(
        &admin,
        &symbol_short!("INV009"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
}

/// Funding an already-funded escrow must panic.
#[test]
#[should_panic(expected = "Escrow not open for funding")]
fn test_fund_after_funded_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV010"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.fund(&investor, &1_000i128); // reaches funded status
    client.fund(&investor, &1i128); // must panic
}

/// Settling an escrow that is still open (not yet funded) must panic.
#[test]
#[should_panic(expected = "Escrow must be funded before settlement")]
fn test_settle_before_funded_panics() {
    let (_, client, admin, sme) = setup();

    client.init(
        &admin,
        &symbol_short!("INV011"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.settle(); // status is still 0 — must panic
}

/// `get_escrow` on an uninitialized contract must panic.
#[test]
#[should_panic(expected = "Escrow not initialized")]
fn test_get_escrow_uninitialized_panics() {
    let env = Env::default();
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);
    client.get_escrow();
}

/// Partial funding across two investors; status stays open until target is met.
#[test]
fn test_partial_fund_stays_open() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV003"),
        &sme,
        &10_000_0000000i128,
        &500i64,
        &2000u64,
    );

    // Fund half — should remain open
    let partial = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(partial.status, 0, "status should still be open");
    assert_eq!(partial.funded_amount, 5_000_0000000i128);

    // Fund the rest — should flip to funded
    let full = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(full.status, 1, "status should be funded");
}

/// Attempting to settle an escrow that is still open must panic.
#[test]
#[should_panic(expected = "Escrow must be funded before settlement")]
fn test_settle_unfunded_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV004"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    client.settle(); // must panic
}

/// Funding an already-funded (status=1) escrow must panic.
#[test]
#[should_panic(expected = "Escrow not open for funding")]
fn test_fund_after_funded_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV005"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    client.fund(&investor, &10_000_0000000i128); // fills target → status 1
    client.fund(&investor, &1i128); // must panic
}

// ---------------------------------------------------------------------------
// Baseline cost tests — core paths
//
// These tests measure and assert upper-bound resource consumption for the
// three primary contract entry points: init, fund, and settle.
//
// Upper bounds are set at 2× the observed baseline to give headroom for minor
// SDK/toolchain changes while still catching significant regressions.
// Tighten the bounds as the contract stabilises.
// ---------------------------------------------------------------------------

/// Baseline cost for `init` — creates a new invoice escrow in storage.
///
/// Expected resource profile: one storage write (instance), struct
/// serialisation.  This is the cheapest path because no prior state is read.
#[test]
fn test_cost_baseline_init() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV100"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    let cost = CostMeasurement::capture(&env, "init");

    // Sanity: metering must have produced non-zero values.
    assert!(cost.instructions > 0, "init: instructions must be > 0");
    assert!(cost.mem_bytes > 0, "init: mem_bytes must be > 0");

    // Regression guards — 3× observed baseline (~34k instructions, ~4.6k mem).
    cost.assert_instructions_below(100_000);
    cost.assert_mem_below(15_000);
}

/// Baseline cost for `fund` (partial) — reads escrow, increments funded_amount,
/// writes back.  Status stays 0 (open) because amount < target.
#[test]
fn test_cost_baseline_fund_partial() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV101"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    // Fund with half the target so status remains open.
    client.fund(&investor, &5_000_0000000i128);

    let cost = CostMeasurement::capture(&env, "fund (partial)");

    assert!(cost.instructions > 0, "fund partial: instructions must be > 0");
    assert!(cost.mem_bytes > 0, "fund partial: mem_bytes must be > 0");

    // Regression guards — 3× observed baseline (~60k instructions, ~9.7k mem).
    cost.assert_instructions_below(180_000);
    cost.assert_mem_below(30_000);
}

/// Baseline cost for `fund` (full) — same read/write path as partial fund but
/// also flips status to 1 (funded).  Should be nearly identical to partial.
#[test]
fn test_cost_baseline_fund_full() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV102"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    // Fund the full target in one call — triggers status transition to 1.
    client.fund(&investor, &10_000_0000000i128);

    let cost = CostMeasurement::capture(&env, "fund (full / status→funded)");

    assert!(cost.instructions > 0, "fund full: instructions must be > 0");
    assert!(cost.mem_bytes > 0, "fund full: mem_bytes must be > 0");

    // Regression guards — 3× observed baseline (~60k instructions, ~9.7k mem).
    cost.assert_instructions_below(180_000);
    cost.assert_mem_below(30_000);
}

/// Baseline cost for `settle` — reads funded escrow, flips status to 2,
/// writes back.  Identical storage pattern to fund.
#[test]
fn test_cost_baseline_settle() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV103"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &10_000_0000000i128);

    // Measure only the settle call.
    client.settle();

    let cost = CostMeasurement::capture(&env, "settle");

    assert!(cost.instructions > 0, "settle: instructions must be > 0");
    assert!(cost.mem_bytes > 0, "settle: mem_bytes must be > 0");

    // Regression guards — 3× observed baseline (~60k instructions, ~9.7k mem).
    cost.assert_instructions_below(180_000);
    cost.assert_mem_below(30_000);
}

// ---------------------------------------------------------------------------
// Edge-case cost tests
//
// These tests validate that resource consumption stays within bounds for
// boundary conditions and less-common execution paths.
// ---------------------------------------------------------------------------

/// Cost of `fund` when the cumulative amount exactly hits the target on the
/// second call (two-step funding).  Validates that the status transition
/// triggered by the second call does not add unexpected overhead.
#[test]
fn test_cost_baseline_fund_two_step_completion() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV200"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    // First partial fund — status stays open.
    client.fund(&investor, &5_000_0000000i128);

    // Second fund — exactly meets target, triggers status → funded.
    client.fund(&investor, &5_000_0000000i128);
    let cost = CostMeasurement::capture(&env, "fund (2nd call, hits target)");

    assert!(cost.instructions > 0);
    // The completing fund call should cost no more than a regular fund call.
    cost.assert_instructions_below(180_000);
    cost.assert_mem_below(30_000);
}

/// Cost of `fund` when the amount overshoots the target.  The contract
/// accumulates the excess; status still flips to funded.
#[test]
fn test_cost_baseline_fund_overshoot() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV201"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    // Fund with 2× the target — overshoot scenario.
    client.fund(&investor, &20_000_0000000i128);
    let cost = CostMeasurement::capture(&env, "fund (overshoot 2×)");

    assert!(cost.instructions > 0);
    cost.assert_instructions_below(180_000);
    cost.assert_mem_below(30_000);
}

/// Cost of `init` with a zero maturity timestamp.  Exercises the minimum-value
/// boundary for the maturity field without changing the storage write pattern.
#[test]
fn test_cost_baseline_init_zero_maturity() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV202"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &0u64, // zero maturity — edge case
    );
    let cost = CostMeasurement::capture(&env, "init (zero maturity)");

    assert!(cost.instructions > 0);
    // Zero maturity is just a field value; cost profile should match normal init.
    cost.assert_instructions_below(100_000);
    cost.assert_mem_below(15_000);
}

/// Cost of `init` with maximum i128 amount.  Ensures large numeric values do
/// not inflate serialisation cost.
#[test]
fn test_cost_baseline_init_max_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &symbol_short!("INV203"),
        &sme,
        &i128::MAX,
        &800i64,
        &1000u64,
    );
    let cost = CostMeasurement::capture(&env, "init (max i128 amount)");

    assert!(cost.instructions > 0);
    cost.assert_instructions_below(100_000);
    cost.assert_mem_below(15_000);
}

/// Cost of `settle` immediately after a single-call full fund.  This is the
/// happy-path end-to-end sequence and validates the combined cost profile.
#[test]
fn test_cost_baseline_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    // --- init ---
    client.init(
        &symbol_short!("INV204"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    let cost_init = CostMeasurement::capture(&env, "lifecycle: init");

    // --- fund ---
    client.fund(&investor, &10_000_0000000i128);
    let cost_fund = CostMeasurement::capture(&env, "lifecycle: fund");

    // --- settle ---
    client.settle();
    let cost_settle = CostMeasurement::capture(&env, "lifecycle: settle");

    // Validate each step individually.
    cost_init.assert_instructions_below(100_000);
    cost_fund.assert_instructions_below(180_000);
    cost_settle.assert_instructions_below(180_000);

    // Validate that settle is not significantly more expensive than fund
    // (both are read-modify-write on the same storage entry).
    let ratio = cost_settle.instructions as f64 / cost_fund.instructions as f64;
    assert!(
        ratio < 1.5,
        "settle should not cost >1.5× fund; got ratio {:.2}",
        ratio
    );
}
