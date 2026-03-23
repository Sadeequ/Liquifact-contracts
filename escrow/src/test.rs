#![cfg(test)]

use super::{LiquifactEscrow, LiquifactEscrowClient};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

fn setup_test(env: &Env) -> (LiquifactEscrowClient<'_>, Address, Symbol) {
    let contract_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(env, &contract_id);
    let sme = Address::generate(env);
    // Use underscore instead of hyphen as symbols only allow a-z, A-Z, 0-9, and _
    let invoice_id = Symbol::new(env, "INV_001");
    (client, sme, invoice_id)
}

#[test]
fn test_initialization_success() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    
    let amount = 1000i128;
    client.init(&id, &sme, &amount, &800, &10000);
    
    let escrow = client.get_escrow();
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.status, 0);
}

#[test]
#[should_panic(expected = "Escrow amount must be positive")]
fn test_init_with_zero_fails() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    client.init(&id, &sme, &0, &800, &10000);
}

#[test]
fn test_funding_success() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    let investor = Address::generate(&env);
    
    client.init(&id, &sme, &1000, &800, &10000);
    client.fund(&investor, &500);
    
    let escrow = client.get_escrow();
    assert_eq!(escrow.funded_amount, 500);
    assert_eq!(escrow.status, 0);
}

#[test]
#[should_panic(expected = "Funding amount must be positive")]
fn test_fund_with_zero_fails() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    let investor = Address::generate(&env);
    
    client.init(&id, &sme, &1000, &800, &10000);
    client.fund(&investor, &0);
}

#[test]
#[should_panic(expected = "Funding amount must be positive")]
fn test_fund_with_negative_fails() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    let investor = Address::generate(&env);
    
    client.init(&id, &sme, &1000, &800, &10000);
    client.fund(&investor, &-100);
}

#[test]
fn test_full_funding_updates_status() {
    let env = Env::default();
    let (client, sme, id) = setup_test(&env);
    let investor = Address::generate(&env);
    
    client.init(&id, &sme, &1000, &800, &10000);
    client.fund(&investor, &1000);
    
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 1); // Status 1 = Funded
}