use super::{
    DataKey, InvestorPositionView, InvoiceEscrow, LiquifactEscrow, LiquifactEscrowClient,
    SCHEMA_VERSION,
};
use soroban_sdk::{symbol_short, testutils::Address as _, testutils::Ledger, Address, Env, Symbol};

fn deploy<'a>(env: &'a Env) -> (Address, LiquifactEscrowClient<'a>) {
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(env, &contract_id);
    (contract_id, client)
}

fn setup_escrow<'a>(
    env: &'a Env,
    invoice_id: &Symbol,
    amount: i128,
    yield_bps: u32,
    maturity: u64,
) -> (LiquifactEscrowClient<'a>, Address, Address, Address) {
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (_contract_id, client) = deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let buyer = Address::generate(env);

    client.init(
        &admin, invoice_id, &sme, &buyer, &amount, &yield_bps, &maturity,
    );

    (client, admin, sme, buyer)
}

fn expect_payout(principal: i128, yield_bps: u32, days_held: i128) -> (i128, i128) {
    // gross_yield = principal * (yield_bps / 10_000) * (days_held / 365)
    let numerator = principal * yield_bps as i128 * days_held;
    let denominator = 10_000i128 * 365i128;
    let gross_yield = numerator / denominator;
    let total_payout = principal + gross_yield;
    (gross_yield, total_payout)
}

#[test]
fn test_init_and_get_escrow_and_version() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV001");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, admin, sme, buyer) = setup_escrow(&env, &invoice_id, amount, yield_bps, maturity);

    let stored: InvoiceEscrow = client.get_escrow();
    assert_eq!(stored.invoice_id, invoice_id);
    assert_eq!(stored.admin, admin);
    assert_eq!(stored.sme_address, sme);
    assert_eq!(stored.buyer_address, buyer);
    assert_eq!(stored.amount, amount);
    assert_eq!(stored.funding_target, amount);
    assert_eq!(stored.funded_amount, 0);
    assert_eq!(stored.yield_bps, yield_bps);
    assert_eq!(stored.maturity, maturity);
    assert_eq!(stored.created_at, created_at);
    assert_eq!(stored.status, 0);
    assert!(!stored.is_paid);
    assert_eq!(stored.version, SCHEMA_VERSION);

    assert_eq!(client.get_version(), SCHEMA_VERSION);
}

#[test]
#[should_panic(expected = "Escrow amount must be positive")]
fn test_init_rejects_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let (_, client) = deploy(&env);

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let buyer = Address::generate(&env);
    client.init(
        &admin,
        &symbol_short!("INV002"),
        &sme,
        &buyer,
        &0i128,
        &800u32,
        &1_000u64,
    );
}

#[test]
fn test_fund_and_query_position_before_settlement() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV003");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, _admin, _sme, _buyer) =
        setup_escrow(&env, &invoice_id, amount, yield_bps, maturity);

    let investor = Address::generate(&env);
    let (_, settled_expected_total) = expect_payout(4_000i128, yield_bps, 365i128);

    // Partial funding: status remains open.
    let esc = client.fund(&investor, &4_000i128);
    assert_eq!(esc.status, 0);

    // Query should return expected payouts even before settlement.
    let pos: InvestorPositionView = client.get_investor_position(&invoice_id, &investor);
    assert_eq!(pos.invoice_id, invoice_id);
    assert_eq!(pos.investor, investor);
    assert_eq!(pos.contribution, 4_000i128);
    assert_eq!(pos.claim_status, 0);
    assert!(!pos.claimable);

    let (expected_yield, expected_total) = expect_payout(4_000i128, yield_bps, 365i128);
    assert_eq!(pos.expected_principal, 4_000i128);
    assert_eq!(pos.expected_yield, expected_yield);
    assert_eq!(pos.expected_payout, expected_total);
    assert_eq!(pos.expected_payout, settled_expected_total);

    // Unknown investor should return zeroed position (no panic).
    let other = Address::generate(&env);
    let other_pos: InvestorPositionView = client.get_investor_position(&invoice_id, &other);
    assert_eq!(other_pos.contribution, 0);
    assert_eq!(other_pos.claim_status, 0);
    assert!(!other_pos.claimable);
    assert_eq!(other_pos.expected_yield, 0);
    assert_eq!(other_pos.expected_payout, 0);
}

#[test]
#[should_panic(expected = "Target escrow invoice_id does not match")]
fn test_investor_position_rejects_invoice_mismatch() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV004");
    let maturity = 1_000_000u64 + 365 * 86_400u64;
    let (client, _, _, _) = setup_escrow(&env, &invoice_id, 10_000i128, 800u32, maturity);

    let investor = Address::generate(&env);
    let wrong_invoice = symbol_short!("INV_WRONG");
    let _ = client.get_investor_position(&wrong_invoice, &investor);
}

#[test]
fn test_settle_flow_and_redeem_updates_claim_status() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV005");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, _admin, _sme, _buyer) =
        setup_escrow(&env, &invoice_id, amount, yield_bps, maturity);

    let investor = Address::generate(&env);

    // Fund to reach status=1 (funded).
    client.fund(&investor, &10_000i128);
    let esc = client.get_escrow();
    assert_eq!(esc.status, 1);

    // Buyer confirms payment.
    let after_confirm = client.confirm_payment();
    assert!(after_confirm.is_paid);

    // SME settles.
    let after_settle = client.settle();
    assert_eq!(after_settle.status, 2);

    // Now the position should be claimable.
    let pos_before_redeem: InvestorPositionView =
        client.get_investor_position(&invoice_id, &investor);
    assert!(pos_before_redeem.claimable);
    assert_eq!(pos_before_redeem.claim_status, 0);

    // Redeem and ensure claim status updates.
    let pos_after_redeem: InvestorPositionView = client.redeem(&investor);
    assert_eq!(pos_after_redeem.contribution, 10_000i128);
    assert_eq!(pos_after_redeem.claim_status, 1);
    assert!(!pos_after_redeem.claimable);

    // Query again should show claimed=true.
    let pos_after_query: InvestorPositionView =
        client.get_investor_position(&invoice_id, &investor);
    assert_eq!(pos_after_query.claim_status, 1);
    assert!(!pos_after_query.claimable);
}

#[test]
#[should_panic(expected = "Escrow must be settled before redemption")]
fn test_redeem_rejected_when_not_settled() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV006");
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, _, _, _) = setup_escrow(&env, &invoice_id, 10_000i128, 800u32, maturity);

    let investor = Address::generate(&env);
    client.fund(&investor, &1_000i128);
    let _ = client.redeem(&investor);
}

#[test]
#[should_panic(expected = "Investor already claimed")]
fn test_redeem_rejected_second_time() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV007");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, _, _, _) = setup_escrow(&env, &invoice_id, amount, yield_bps, maturity);

    let investor = Address::generate(&env);
    client.fund(&investor, &amount);
    client.confirm_payment();
    client.settle();

    let _ = client.redeem(&investor);
    let _ = client.redeem(&investor);
}

#[test]
#[should_panic(expected = "from_version does not match stored version")]
fn test_migrate_rejects_wrong_from_version() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (_, client) = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV008"),
        &sme,
        &buyer,
        &10_000i128,
        &800u32,
        &(1_000_000u64 + 365 * 86_400u64),
    );

    // Stored version is SCHEMA_VERSION; mismatch should panic.
    client.migrate(&(SCHEMA_VERSION + 1));
}

#[test]
#[should_panic(expected = "Already at current schema version")]
fn test_migrate_rejects_when_already_current() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (_, client) = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV009"),
        &sme,
        &buyer,
        &10_000i128,
        &800u32,
        &(1_000_000u64 + 365 * 86_400u64),
    );

    client.migrate(&SCHEMA_VERSION);
}

#[test]
#[should_panic(expected = "No contribution for investor")]
fn test_redeem_rejected_when_investor_has_zero_contribution() {
    let env = Env::default();
    let invoice_id = symbol_short!("INV011");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let created_at = 1_000_000u64;
    let maturity = created_at + 365 * 86_400u64;

    let (client, _, _, _) = setup_escrow(&env, &invoice_id, amount, yield_bps, maturity);

    // Fund with another investor, so `investor` below has 0 contribution.
    let funded_investor = Address::generate(&env);
    client.fund(&funded_investor, &amount);
    client.confirm_payment();
    client.settle();

    let investor = Address::generate(&env);
    let _ = client.redeem(&investor);
}

#[test]
#[should_panic(expected = "No migration path from version 0")]
fn test_migrate_reaches_no_path_panic() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (contract_id, client) = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init(
        &admin,
        &symbol_short!("INV012"),
        &sme,
        &buyer,
        &10_000i128,
        &800u32,
        &(1_000_000u64 + 365 * 86_400u64),
    );

    // Simulate an older stored version by directly mutating contract storage.
    let mut stored: InvoiceEscrow = client.get_escrow();
    stored.version = 0;
    let stored_to_set = stored.clone();
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::Escrow, &stored_to_set);
    });

    client.migrate(&0u32);
}

#[test]
fn test_yield_is_zero_when_maturity_is_immediate() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (_, client) = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let buyer = Address::generate(&env);

    let invoice_id = symbol_short!("INV010");
    let amount = 10_000i128;
    let yield_bps = 800u32;
    let maturity = 1_000_000u64; // created_at, so days_held = 0

    client.init(
        &admin,
        &invoice_id,
        &sme,
        &buyer,
        &amount,
        &yield_bps,
        &maturity,
    );

    let investor = Address::generate(&env);
    client.fund(&investor, &5_000i128);

    let pos: InvestorPositionView = client.get_investor_position(&invoice_id, &investor);
    assert_eq!(pos.expected_yield, 0);
    assert_eq!(pos.expected_payout, 5_000i128);
}
