//! LiquiFact Escrow Contract
//!
//! Holds investor funds for an invoice until settlement.
//! - SME receives stablecoin when funding target is met
//! - Investors receive principal + yield when buyer pays at maturity

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceEscrow {
    pub invoice_id: Symbol,
    pub sme_address: Address,
    pub amount: i128,
    pub funding_target: i128,
    pub funded_amount: i128,
    pub yield_bps: i64,
    pub maturity: u64,

    /// NEW: deadline for funding
    pub funding_deadline: u64,

    /// 0 = open, 1 = funded, 2 = settled, 3 = expired
    pub status: u32,
}

#[contract]
pub struct LiquifactEscrow;

#[contractimpl]
impl LiquifactEscrow {
    pub fn init(
        env: Env,
        invoice_id: Symbol,
        sme_address: Address,
        amount: i128,
        yield_bps: i64,
        maturity: u64,
        funding_deadline: u64, // NEW
    ) -> InvoiceEscrow {
        if env.storage().instance().has(&symbol_short!("escrow")) {
            panic!("Escrow already initialized");
        }

        let now = env.ledger().timestamp();

        // validations
        assert!(amount > 0, "Amount must be positive");
        assert!(yield_bps >= 0, "Invalid yield");
        assert!(funding_deadline > now, "Invalid funding deadline");
        assert!(maturity > funding_deadline, "Maturity must be after funding deadline");

        let escrow = InvoiceEscrow {
            invoice_id: invoice_id.clone(),
            sme_address: sme_address.clone(),
            amount,
            funding_target: amount,
            funded_amount: 0,
            yield_bps,
            maturity,
            funding_deadline,
            status: 0,
        };

        env.storage().instance().set(&symbol_short!("escrow"), &escrow);
        escrow
    }

    /// Get current escrow state.
     pub fn get_escrow(env: Env) -> InvoiceEscrow {
        env.storage()
            .instance()
            .get(&symbol_short!("escrow"))
            .unwrap_or_else(|| panic!("Escrow not initialized"))
    }

    /// INTERNAL: expire escrow if deadline passed
    fn check_and_update_expiry(env: &Env, escrow: &mut InvoiceEscrow) {
        let now = env.ledger().timestamp();

        if escrow.status == 0 && now > escrow.funding_deadline {
            escrow.status = 3; // expired
        }
    }

    /// Record investor funding. In production, this would be called with token transfer.
    pub fn fund(env: Env, investor: Address, amount: i128) -> InvoiceEscrow {
        investor.require_auth();

        let mut escrow = Self::get_escrow(env.clone());

        // check expiry first
        Self::check_and_update_expiry(&env, &mut escrow);

        assert!(escrow.status == 0, "Escrow not open for funding");
        assert!(amount > 0, "Invalid funding amount");

        escrow.funded_amount = escrow
            .funded_amount
            .checked_add(amount)
            .expect("Overflow");

        if escrow.funded_amount >= escrow.funding_target {
            escrow.status = 1; // funded
        }

        env.storage().instance().set(&symbol_short!("escrow"), &escrow);
        escrow
    }

    pub fn settle(env: Env) -> InvoiceEscrow {
        let mut escrow = Self::get_escrow(env.clone());

        // check expiry
        Self::check_and_update_expiry(&env, &mut escrow);

        assert!(escrow.status == 1, "Escrow must be funded");

        assert!(
            env.ledger().timestamp() >= escrow.maturity,
            "Cannot settle before maturity"
        );

        escrow.status = 2;

        env.storage().instance().set(&symbol_short!("escrow"), &escrow);
        escrow
    }
}

#[cfg(test)]
mod test;
