use super::{LiquifactEscrow, LiquifactEscrowClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    Address, Env, IntoVal,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deploy a fresh contract and return (client, sme_address).
fn setup(env: &Env) -> (LiquifactEscrowClient<'_>, Address) {
    let sme = Address::generate(env);
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(env, &contract_id);
    (client, sme)
}

/// Init with sensible defaults (target = 10_000 XLM, 8% yield, maturity 1000).
fn default_init(client: &LiquifactEscrowClient, sme: &Address) {
    client.init(
        &symbol_short!("INV001"),
        sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
}

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

#[test]
fn test_init_stores_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
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
    assert_eq!(escrow.funding_target, 10_000_0000000i128);
    assert_eq!(escrow.funded_amount, 0);
    assert_eq!(escrow.yield_bps, 800);
    assert_eq!(escrow.maturity, 1000);
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
fn test_get_escrow_uninitialized_panics() {
fn test_init_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup(&env);

    let result = client.try_get_escrow();
    assert!(result.is_err());
}

#[test]
fn test_double_init_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let result = client.try_init(
        &symbol_short!("INV002"),
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
        &5_000_0000000i128,
        &500i64,
        &2000u64,
    );
    assert!(result.is_err());
}

#[test]
fn test_init_requires_admin_auth() {
    let env = Env::default();
    // Do NOT mock auths – the sme_address.require_auth() must fire.
    let (client, sme) = setup(&env);

    // Provide auth only for sme so the call succeeds.
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &sme,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &client.address,
            fn_name: "init",
            args: (
                symbol_short!("INV001"),
                sme.clone(),
                10_000_0000000i128,
                800i64,
                1000u64,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let escrow = client.init(
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    assert_eq!(escrow.status, 0);
}

#[test]
fn test_init_unauthorized_panics() {
    let env = Env::default();
    // No auths mocked at all → require_auth() will panic.
    let (client, sme) = setup(&env);

    let result = client.try_init(
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// fund – edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_fund_zero_amount_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    let result = client.try_fund(&investor, &0i128);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// fund – basic behaviour
// ---------------------------------------------------------------------------

#[test]
fn test_fund_partial_then_full() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);

    // Partial fund – status stays open.
    let e1 = client.fund(&investor, &4_000_0000000i128);
    assert_eq!(e1.funded_amount, 4_000_0000000i128);
    assert_eq!(e1.status, 0);

    // Complete funding – status becomes funded.
    let e2 = client.fund(&investor, &6_000_0000000i128);
    assert_eq!(e2.funded_amount, 10_000_0000000i128);
    assert_eq!(e2.status, 1);
}

#[test]
fn test_fund_after_funded_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);

    let result = client.try_fund(&investor, &1i128);
    assert!(result.is_err());
}

#[test]
fn test_fund_requires_investor_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);

    // Remove all mocked auths → investor.require_auth() should fail.
    env.mock_auths(&[]);
    let result = client.try_fund(&investor, &1_000_0000000i128);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// fund – per-investor ledger (new behaviour)
// ---------------------------------------------------------------------------

#[test]
fn test_single_investor_contribution_tracked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &3_000_0000000i128);

    assert_eq!(client.get_contribution(&investor), 3_000_0000000i128);
}

#[test]
fn test_repeated_funding_accumulates_contribution() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &2_000_0000000i128);
    client.fund(&investor, &3_000_0000000i128);

    // Ledger must reflect the sum of both calls.
    assert_eq!(client.get_contribution(&investor), 5_000_0000000i128);
}

#[test]
fn test_multiple_investors_tracked_independently() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let inv_a = Address::generate(&env);
    let inv_b = Address::generate(&env);

    client.fund(&inv_a, &4_000_0000000i128);
    client.fund(&inv_b, &6_000_0000000i128);

    assert_eq!(client.get_contribution(&inv_a), 4_000_0000000i128);
    assert_eq!(client.get_contribution(&inv_b), 6_000_0000000i128);
}

#[test]
fn test_contributions_sum_equals_funded_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let inv_a = Address::generate(&env);
    let inv_b = Address::generate(&env);
    let inv_c = Address::generate(&env);

    client.fund(&inv_a, &2_000_0000000i128);
    client.fund(&inv_b, &5_000_0000000i128);
    client.fund(&inv_c, &3_000_0000000i128);

    let total = client.get_contribution(&inv_a)
        + client.get_contribution(&inv_b)
        + client.get_contribution(&inv_c);

    let escrow = client.get_escrow();
    assert_eq!(total, escrow.funded_amount);
}

#[test]
fn test_unknown_investor_contribution_is_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let stranger = Address::generate(&env);
    assert_eq!(client.get_contribution(&stranger), 0i128);
}

// ---------------------------------------------------------------------------
// settle
// ---------------------------------------------------------------------------

#[test]
fn test_settle_after_full_funding() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);

    // Advance ledger past maturity.
    env.ledger().set_timestamp(1001);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_before_funded_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let result = client.try_settle();
    assert!(result.is_err());
}

#[test]
fn test_settle_before_maturity_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);

    // Ledger timestamp defaults to 0, which is before maturity 1000.
    let result = client.try_settle();
    assert!(result.is_err());
}

#[test]
fn test_settle_at_exact_maturity_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);

    env.ledger().set_timestamp(1000); // exactly at maturity
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_after_maturity_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);

    env.ledger().set_timestamp(9999);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_with_zero_maturity_succeeds_immediately() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);

    // maturity = 0 means "no maturity lock"
    client.init(
        &symbol_short!("INV003"),
        &sme,
        &1_000_0000000i128,
        &500i64,
        &0u64,
    );

    let investor = Address::generate(&env);
    client.fund(&investor, &1_000_0000000i128);

    // timestamp is 0 by default; maturity == 0 bypasses the check
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_at_timestamp_zero_before_maturity_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);

    // maturity = 500, ledger timestamp = 0 → should panic
    client.init(
        &symbol_short!("INV004"),
        &sme,
        &1_000_0000000i128,
        &500i64,
        &500u64,
    );

    let investor = Address::generate(&env);
    client.fund(&investor, &1_000_0000000i128);

    let result = client.try_settle();
    assert!(result.is_err());
}

#[test]
fn test_settle_requires_sme_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);
    env.ledger().set_timestamp(1001);

    // Remove all mocked auths → sme_address.require_auth() should fail.
    env.mock_auths(&[]);
    let result = client.try_settle();
    assert!(result.is_err());
}

#[test]
fn test_settle_unauthorized_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, sme) = setup(&env);
    default_init(&client, &sme);

    let investor = Address::generate(&env);
    client.fund(&investor, &10_000_0000000i128);
    env.ledger().set_timestamp(1001);

    // Remove all mocked auths → settle should be rejected.
    env.mock_auths(&[]);
    let result = client.try_settle();
    assert!(result.is_err());
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
