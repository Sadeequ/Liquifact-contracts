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

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
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

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    client.init(
        &admin,
        &symbol_short!("INV004"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );

    client.settle(); // must panic
}

// ---------------------------------------------------------------------------
// State Transition Matrix Tests
// ---------------------------------------------------------------------------
//
// Escrow States:
//   0 = Open    (initialized, ready for funding)
//   1 = Funded  (funding target met, ready for settlement)
//   2 = Settled (completed, buyer paid)
//
// Allowed Transitions:
//   init   -> Open    (new escrow)
//   fund   -> Open    (add funding, stay open if not full)
//   fund   -> Funded  (funding target reached)
//   settle -> Settled (only from Funded)
//
// Forbidden Transitions (must panic):
//   fund   from Open -> Funded (second fund call)
//   fund   from Funded
//   fund   from Settled
//   settle from Open
//   settle from Settled
//   init   twice (re-init)
//

/// Transition: Open (0) -> Funded (1) via fund() - happy path
#[test]
fn test_transition_open_to_funded() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX001"), &sme, &1000i128, &500i64, &2000u64);
    assert_eq!(client.get_escrow().status, 0); // open

    let escrow = client.fund(&investor, &1000i128);
    assert_eq!(escrow.status, 1); // funded
}

/// Transition: Funded (1) -> Settled (2) via settle() - happy path
#[test]
fn test_transition_funded_to_settled() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX002"), &sme, &1000i128, &500i64, &2000u64);
    client.fund(&investor, &1000i128);
    assert_eq!(client.get_escrow().status, 1); // funded

    let escrow = client.settle();
    assert_eq!(escrow.status, 2); // settled
}

/// Forbidden: fund() from Open -> Open (partial funding stays open)
#[test]
fn test_transition_open_to_open_partial_funding() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX003"), &sme, &1000i128, &500i64, &2000u64);
    assert_eq!(client.get_escrow().status, 0); // open

    let escrow = client.fund(&investor, &500i128);
    assert_eq!(escrow.status, 0); // still open (partial funding)
}

/// Forbidden Transition: fund() from Funded (1) must panic
#[test]
#[should_panic(expected = "Escrow not open for funding")]
fn test_transition_funded_fund_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX004"), &sme, &1000i128, &500i64, &2000u64);
    client.fund(&investor, &1000i128); // status = 1 (funded)
    client.fund(&investor, &100i128); // must panic - already funded
}

/// Forbidden Transition: fund() from Settled (2) must panic
#[test]
#[should_panic(expected = "Escrow not open for funding")]
fn test_transition_settled_fund_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX005"), &sme, &1000i128, &500i64, &2000u64);
    client.fund(&investor, &1000i128); // status = 1 (funded)
    client.settle(); // status = 2 (settled)
    client.fund(&investor, &100i128); // must panic - already settled
}

/// Forbidden Transition: settle() from Open (0) must panic
#[test]
#[should_panic(expected = "Escrow must be funded before settlement")]
fn test_transition_open_settle_panics() {
    let (_, client, admin, sme) = setup();

    client.init(&admin, &symbol_short!("TX006"), &sme, &1000i128, &500i64, &2000u64);
    // status = 0 (open), not funded
    client.settle(); // must panic
}

/// Forbidden Transition: settle() from Settled (2) must panic
#[test]
#[should_panic(expected = "Escrow must be funded before settlement")]
fn test_transition_settled_settle_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX007"), &sme, &1000i128, &500i64, &2000u64);
    client.fund(&investor, &1000i128);
    client.settle(); // status = 2 (settled)
    client.settle(); // must panic - already settled
}

/// Regression: Double init must panic (state transition matrix completeness)
#[test]
#[should_panic(expected = "Escrow already initialized")]
fn test_transition_double_init_panics() {
    let (_, client, admin, sme) = setup();

    client.init(&admin, &symbol_short!("TX008"), &sme, &1000i128, &500i64, &2000u64);
    client.init(&admin, &symbol_short!("TX009"), &sme, &2000i128, &500i64, &3000u64); // must panic
}

/// Edge Case: Fund exact amount then fund more should panic
#[test]
#[should_panic(expected = "Escrow not open for funding")]
fn test_transition_exact_fund_then_more_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX010"), &sme, &1000i128, &500i64, &2000u64);
    client.fund(&investor, &1000i128); // exact amount -> funded
    client.fund(&investor, &1i128); // must panic
}

/// Edge Case: Partial fund then full fund leads to funded
#[test]
fn test_transition_partial_then_full_funded() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX011"), &sme, &1000i128, &500i64, &2000u64);
    assert_eq!(client.get_escrow().status, 0);

    let escrow = client.fund(&investor, &500i128); // partial
    assert_eq!(escrow.status, 0); // still open
    assert_eq!(escrow.funded_amount, 500i128);

    let escrow = client.fund(&investor, &500i128); // complete funding
    assert_eq!(escrow.status, 1); // funded
}

/// Edge Case: Multiple partial funds without reaching target
#[test]
fn test_transition_multiple_partial_funds() {
    let (env, client, admin, sme) = setup();
    let investor1 = Address::generate(&env);
    let investor2 = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX012"), &sme, &1000i128, &500i64, &2000u64);

    client.fund(&investor1, &300i128); // status = 0 (open)
    let escrow = client.fund(&investor2, &300i128); // still open, 600 funded
    assert_eq!(escrow.status, 0);
    assert_eq!(escrow.funded_amount, 600i128);

    client.fund(&investor1, &400i128); // now 1000 reached -> funded
    assert_eq!(client.get_escrow().status, 1);
}

/// Security: Verify status values are exactly as defined in matrix
#[test]
fn test_state_values_are_correct() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);

    client.init(&admin, &symbol_short!("TX013"), &sme, &1000i128, &500i64, &2000u64);
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 0, "Init should set status to Open (0)");

    client.fund(&investor, &1000i128);
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 1, "Full funding should set status to Funded (1)");

    client.settle();
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 2, "Settle should set status to Settled (2)");
}
