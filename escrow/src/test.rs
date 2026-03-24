use super::{LiquifactEscrow, LiquifactEscrowClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    Address, Env,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
// init
// ---------------------------------------------------------------------------

#[test]
fn test_init_and_get_escrow() {
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
    let got = client.get_escrow();
    assert_eq!(got.invoice_id, escrow.invoice_id);
    assert_eq!(got.admin, admin);
}

#[test]
fn test_init_stores_escrow() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 0);
    assert_eq!(escrow.funded_amount, 0);
}

#[test]
#[should_panic(expected = "Escrow already initialized")]
fn test_double_init_panics() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
}

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
    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == admin),
        "admin auth not recorded"
    );
}

#[test]
#[should_panic]
fn test_init_unauthorized_panics() {
    let env = Env::default();
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

// ---------------------------------------------------------------------------
// get_escrow
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Escrow not initialized")]
fn test_get_escrow_uninitialized_panics() {
    let env = Env::default();
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(&env, &contract_id);
    client.get_escrow();
}

// ---------------------------------------------------------------------------
// fund
// ---------------------------------------------------------------------------

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
    let e1 = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(e1.funded_amount, 5_000_0000000i128);
    assert_eq!(e1.status, 0);
    let e2 = client.fund(&investor, &5_000_0000000i128);
    assert_eq!(e2.funded_amount, 10_000_0000000i128);
    assert_eq!(e2.status, 1);
}

#[test]
fn test_fund_and_settle() {
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
    env.ledger().set_timestamp(1001);
    let settled = client.settle();
    assert_eq!(settled.status, 2);
}

#[test]
#[should_panic]
fn test_fund_zero_amount_panics() {
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
    client.fund(&investor, &0i128);
}

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
    client.fund(&investor, &1_000i128);
    client.fund(&investor, &1i128);
}

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
        "investor auth not recorded"
    );
}

// ---------------------------------------------------------------------------
// Per-investor ledger
// ---------------------------------------------------------------------------

#[test]
fn test_single_investor_contribution_tracked() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV020"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &3_000_0000000i128);
    assert_eq!(client.get_contribution(&investor), 3_000_0000000i128);
}

#[test]
fn test_repeated_funding_accumulates_contribution() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV021"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &2_000_0000000i128);
    client.fund(&investor, &3_000_0000000i128);
    assert_eq!(client.get_contribution(&investor), 5_000_0000000i128);
}

#[test]
fn test_multiple_investors_tracked_independently() {
    let (env, client, admin, sme) = setup();
    let inv_a = Address::generate(&env);
    let inv_b = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV022"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&inv_a, &4_000_0000000i128);
    client.fund(&inv_b, &6_000_0000000i128);
    assert_eq!(client.get_contribution(&inv_a), 4_000_0000000i128);
    assert_eq!(client.get_contribution(&inv_b), 6_000_0000000i128);
}

#[test]
fn test_contributions_sum_equals_funded_amount() {
    let (env, client, admin, sme) = setup();
    let inv_a = Address::generate(&env);
    let inv_b = Address::generate(&env);
    let inv_c = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV023"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
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
    let (env, client, admin, sme) = setup();
    let stranger = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV024"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    assert_eq!(client.get_contribution(&stranger), 0i128);
}

// ---------------------------------------------------------------------------
// settle
// ---------------------------------------------------------------------------

#[test]
fn test_settle_after_full_funding() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV030"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &10_000_0000000i128);
    env.ledger().set_timestamp(1001);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
#[should_panic(expected = "Escrow must be funded before settlement")]
fn test_settle_before_funded_panics() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV031"),
        &sme,
        &1_000i128,
        &500i64,
        &2000u64,
    );
    client.settle();
}

#[test]
fn test_settle_before_maturity_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV032"),
        &sme,
        &1_000i128,
        &500i64,
        &1000u64,
    );
    client.fund(&investor, &1_000i128);
    let result = client.try_settle();
    assert!(result.is_err());
}

#[test]
fn test_settle_at_exact_maturity_succeeds() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV033"),
        &sme,
        &1_000i128,
        &500i64,
        &1000u64,
    );
    client.fund(&investor, &1_000i128);
    env.ledger().set_timestamp(1000);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_after_maturity_succeeds() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV034"),
        &sme,
        &1_000i128,
        &500i64,
        &1000u64,
    );
    client.fund(&investor, &1_000i128);
    env.ledger().set_timestamp(9999);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_with_zero_maturity_succeeds_immediately() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV035"),
        &sme,
        &1_000_0000000i128,
        &500i64,
        &0u64,
    );
    client.fund(&investor, &1_000_0000000i128);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

#[test]
fn test_settle_at_timestamp_zero_before_maturity_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV036"),
        &sme,
        &1_000i128,
        &500i64,
        &500u64,
    );
    client.fund(&investor, &1_000i128);
    let result = client.try_settle();
    assert!(result.is_err());
}

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
        &0u64,
    );
    client.fund(&investor, &1_000i128);
    client.settle();
    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == sme),
        "sme auth not recorded"
    );
}

#[test]
#[should_panic]
fn test_settle_unauthorized_panics() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV008"),
        &sme,
        &1_000i128,
        &500i64,
        &0u64,
    );
    client.fund(&investor, &1_000i128);
    env.mock_auths(&[]);
    client.settle();
}

// ---------------------------------------------------------------------------
// Cost baselines
// ---------------------------------------------------------------------------

#[test]
fn test_cost_baseline_init() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
}

#[test]
fn test_cost_baseline_init_zero_maturity() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &0u64,
    );
}

#[test]
fn test_cost_baseline_init_max_amount() {
    let (_, client, admin, sme) = setup();
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &i128::MAX,
        &800i64,
        &1000u64,
    );
}

#[test]
fn test_cost_baseline_fund_partial() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &1_000_0000000i128);
}

#[test]
fn test_cost_baseline_fund_full() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &10_000_0000000i128);
}

#[test]
fn test_cost_baseline_fund_overshoot() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &20_000_0000000i128);
}

#[test]
fn test_cost_baseline_fund_two_step_completion() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &5_000_0000000i128);
    client.fund(&investor, &5_000_0000000i128);
}

#[test]
fn test_cost_baseline_settle() {
    let (env, client, admin, sme) = setup();
    let investor = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&investor, &10_000_0000000i128);
    env.ledger().set_timestamp(1001);
    client.settle();
}

#[test]
fn test_cost_baseline_full_lifecycle() {
    let (env, client, admin, sme) = setup();
    let inv_a = Address::generate(&env);
    let inv_b = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV001"),
        &sme,
        &10_000_0000000i128,
        &800i64,
        &1000u64,
    );
    client.fund(&inv_a, &6_000_0000000i128);
    client.fund(&inv_b, &4_000_0000000i128);
    env.ledger().set_timestamp(1001);
    client.settle();
}
