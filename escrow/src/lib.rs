#![no_std]
//! LiquiFact Escrow contract.
//!
//! Holds investor funds for an invoice until settlement, tracks investor
//! contributions, and exposes a read-only query method for investor positions.

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Current storage schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Storage key for escrow state and per-investor records.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Escrow,
    Investor(Address),
}

/// Full escrow state persisted under [`DataKey::Escrow`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceEscrow {
    /// Unique invoice identifier (Soroban `Symbol`, max 12 chars).
    pub invoice_id: Symbol,
    /// Contract admin address (can evolve schema in future versions).
    pub admin: Address,
    /// SME wallet that receives liquidity and authorizes settlement.
    pub sme_address: Address,
    /// Buyer address that confirms repayment.
    pub buyer_address: Address,

    /// Total invoice amount in smallest token units.
    pub amount: i128,
    /// Investor funding target. For now, equals [`amount`].
    pub funding_target: i128,
    /// Running total committed by investors so far.
    pub funded_amount: i128,

    /// Investor yield basis points (e.g. `800` = 8%).
    pub yield_bps: u32,
    /// Ledger timestamp (Unix seconds) at which the invoice matures.
    pub maturity: u64,
    /// Contract creation timestamp (Unix seconds).
    pub created_at: u64,

    /// Escrow lifecycle status:
    /// - `0` — open
    /// - `1` — funded
    /// - `2` — settled
    pub status: u32,
    /// Whether the buyer has confirmed payment.
    pub is_paid: bool,

    /// Storage schema version.
    pub version: u32,
}

/// Persisted investor accounting (contribution + redemption flag).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvestorRecord {
    /// Total contribution made by the investor.
    pub contribution: i128,
    /// Whether the investor has redeemed principal + yield.
    pub claimed: bool,
}

/// Read-only view returned by [`LiquifactEscrow::get_investor_position`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvestorPositionView {
    /// Target invoice id that this position belongs to.
    pub invoice_id: Symbol,
    /// Investor wallet.
    pub investor: Address,

    /// Investor contribution (principal).
    pub contribution: i128,
    /// Claim status: `0` = unclaimed, `1` = claimed.
    pub claim_status: u32,
    /// Whether the investor can redeem right now.
    pub claimable: bool,

    /// Principal amount the investor is entitled to redeem.
    pub expected_principal: i128,
    /// Expected yield amount (computed from [`InvoiceEscrow::yield_bps`] and term).
    pub expected_yield: i128,
    /// Expected total payout = principal + expected_yield.
    pub expected_payout: i128,
}

// ─────────────────────────────────────────────────────────────────────────────
// Events
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted by `init()`.
#[contractevent]
pub struct EscrowInitialized {
    #[topic]
    pub name: Symbol,
    pub invoice_id: Symbol,
    pub sme_address: Address,
    pub buyer_address: Address,
    pub amount: i128,
    pub funding_target: i128,
    pub yield_bps: u32,
    pub maturity: u64,
    pub status: u32,
}

/// Emitted by `fund()` on every successful contribution call.
#[contractevent]
pub struct EscrowFunded {
    #[topic]
    pub name: Symbol,
    pub invoice_id: Symbol,
    pub investor: Address,
    pub amount: i128,
    pub funded_amount: i128,
    pub status: u32,
}

/// Emitted by `settle()` when the escrow transitions to `status = 2`.
#[contractevent]
pub struct EscrowSettled {
    #[topic]
    pub name: Symbol,
    pub invoice_id: Symbol,
    pub funded_amount: i128,
    pub yield_bps: u32,
    pub maturity: u64,
}

/// Emitted by `redeem()` when an investor redeems.
#[contractevent]
pub struct InvestorRedeemed {
    #[topic]
    pub name: Symbol,
    pub invoice_id: Symbol,
    pub investor: Address,
    pub principal: i128,
    pub yield_amount: i128,
    pub total_payout: i128,
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct LiquifactEscrow;

#[contractimpl]
impl LiquifactEscrow {
    fn escrow_key() -> DataKey {
        DataKey::Escrow
    }

    fn investor_key(investor: Address) -> DataKey {
        DataKey::Investor(investor)
    }

    fn load_escrow(env: &Env) -> InvoiceEscrow {
        env.storage()
            .instance()
            .get::<DataKey, InvoiceEscrow>(&Self::escrow_key())
            .unwrap_or_else(|| panic!("Escrow not initialized"))
    }

    fn get_investor_record(env: &Env, investor: Address) -> InvestorRecord {
        env.storage()
            .instance()
            .get::<DataKey, InvestorRecord>(&Self::investor_key(investor.clone()))
            .unwrap_or(InvestorRecord {
                contribution: 0,
                claimed: false,
            })
    }

    fn compute_expected_payout(escrow: &InvoiceEscrow, principal: i128) -> (i128, i128) {
        // Expected yield uses the contract documentation formula:
        // gross_yield = principal * (yield_bps / 10_000) * (days_held / 365)
        //
        // Term length is derived from `maturity` - `created_at`.
        if principal == 0 {
            return (0, 0);
        }

        let days_held: u64 = escrow.maturity.saturating_sub(escrow.created_at) / 86_400u64; // seconds per day

        // Convert to i128 for safe integer math.
        let principal_i = principal;
        let yield_bps_i = escrow.yield_bps as i128;
        let days_held_i = days_held as i128;

        // gross_yield = principal * yield_bps * days / (10_000 * 365)
        let numerator = principal_i
            .checked_mul(yield_bps_i)
            .expect("yield overflow");
        let numerator = numerator.checked_mul(days_held_i).expect("yield overflow");

        let denominator = (10_000i128)
            .checked_mul(365i128)
            .expect("denominator overflow");
        let gross_yield = numerator
            .checked_div(denominator)
            .expect("yield division failed");
        let total = principal_i
            .checked_add(gross_yield)
            .expect("payout overflow");

        (gross_yield, total)
    }

    // ---------------------------------------------------------------------
    // init
    // ---------------------------------------------------------------------

    /// Initialize a new invoice escrow.
    ///
    /// # Panics
    /// - If `amount <= 0`
    /// - If `yield_bps > 10_000`
    /// - If the escrow has already been initialized
    pub fn init(
        env: Env,
        admin: Address,
        invoice_id: Symbol,
        sme_address: Address,
        buyer_address: Address,
        amount: i128,
        yield_bps: u32,
        maturity: u64,
    ) -> InvoiceEscrow {
        assert!(amount > 0, "Escrow amount must be positive");
        assert!(yield_bps <= 10_000, "yield_bps must be <= 10_000");

        let key = Self::escrow_key();
        assert!(
            !env.storage().instance().has(&key),
            "Escrow already initialized"
        );

        let created_at = env.ledger().timestamp();
        let escrow = InvoiceEscrow {
            invoice_id: invoice_id.clone(),
            admin,
            sme_address,
            buyer_address,
            amount,
            funding_target: amount,
            funded_amount: 0,
            yield_bps,
            maturity,
            created_at,
            status: 0,
            is_paid: false,
            version: SCHEMA_VERSION,
        };

        env.storage().instance().set(&key, &escrow);

        EscrowInitialized {
            name: symbol_short!("initd"),
            invoice_id: escrow.invoice_id.clone(),
            sme_address: escrow.sme_address.clone(),
            buyer_address: escrow.buyer_address.clone(),
            amount: escrow.amount,
            funding_target: escrow.funding_target,
            yield_bps: escrow.yield_bps,
            maturity: escrow.maturity,
            status: escrow.status,
        }
        .publish(&env);

        escrow
    }

    // ---------------------------------------------------------------------
    // Basic reads
    // ---------------------------------------------------------------------

    /// Return the current escrow state.
    ///
    /// # Panics
    /// - If `init` has not been called yet.
    pub fn get_escrow(env: Env) -> InvoiceEscrow {
        Self::load_escrow(&env)
    }

    /// Returns the stored schema version.
    pub fn get_version(env: Env) -> u32 {
        Self::load_escrow(&env).version
    }

    /// Migrate storage from an older schema version to the current one.
    ///
    /// This contract currently has no migration paths.
    ///
    /// # Panics
    /// - If `from_version` does not match the stored schema version.
    pub fn migrate(env: Env, from_version: u32) -> u32 {
        let stored_version = Self::load_escrow(&env).version;
        assert!(
            stored_version == from_version,
            "from_version does not match stored version"
        );
        assert!(
            from_version < SCHEMA_VERSION,
            "Already at current schema version"
        );
        panic!("No migration path from version {}", from_version);
    }

    // ---------------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------------

    /// Record investor funding.
    ///
    /// Requires authorization from `investor`.
    ///
    /// # Panics
    /// - If the escrow is not open (`status != 0`)
    /// - If `amount <= 0`
    pub fn fund(env: Env, investor: Address, amount: i128) -> InvoiceEscrow {
        investor.require_auth();

        let mut escrow = Self::load_escrow(&env);
        assert!(amount > 0, "Funding amount must be positive");
        assert!(escrow.status == 0, "Escrow not open for funding");

        // Update escrow total.
        escrow.funded_amount = escrow
            .funded_amount
            .checked_add(amount)
            .expect("funded_amount overflow");

        // Update per-investor contribution.
        let mut record = Self::get_investor_record(&env, investor.clone());
        record.contribution = record
            .contribution
            .checked_add(amount)
            .expect("contribution overflow");

        // Persist.
        env.storage()
            .instance()
            .set(&Self::investor_key(investor.clone()), &record);

        if escrow.funded_amount >= escrow.funding_target {
            escrow.status = 1;
        }

        env.storage().instance().set(&Self::escrow_key(), &escrow);

        EscrowFunded {
            name: symbol_short!("funded"),
            invoice_id: escrow.invoice_id.clone(),
            investor,
            amount,
            funded_amount: escrow.funded_amount,
            status: escrow.status,
        }
        .publish(&env);

        escrow
    }

    /// Buyer confirms repayment.
    ///
    /// # Panics
    /// - If called before the escrow is fully funded
    pub fn confirm_payment(env: Env) -> InvoiceEscrow {
        let mut escrow = Self::load_escrow(&env);
        assert!(
            escrow.status == 1,
            "Escrow must be funded before payment confirmation"
        );
        escrow.buyer_address.require_auth();
        assert!(!escrow.is_paid, "Payment already confirmed");
        escrow.is_paid = true;
        env.storage().instance().set(&Self::escrow_key(), &escrow);
        escrow
    }

    /// Finalize settlement (transition escrow to `status = 2`).
    ///
    /// Requires authorization from the configured SME address.
    pub fn settle(env: Env) -> InvoiceEscrow {
        let mut escrow = Self::load_escrow(&env);
        escrow.sme_address.require_auth();
        assert!(
            escrow.status == 1,
            "Escrow must be funded before settlement"
        );
        assert!(escrow.is_paid, "Payment has not been confirmed yet");

        escrow.status = 2;
        env.storage().instance().set(&Self::escrow_key(), &escrow);

        EscrowSettled {
            name: symbol_short!("settled"),
            invoice_id: escrow.invoice_id.clone(),
            funded_amount: escrow.funded_amount,
            yield_bps: escrow.yield_bps,
            maturity: escrow.maturity,
        }
        .publish(&env);

        escrow
    }

    /// Redeem principal + yield for the given investor.
    ///
    /// This method marks the investor as claimed; it does not perform token
    /// transfers (accounting only).
    pub fn redeem(env: Env, investor: Address) -> InvestorPositionView {
        investor.require_auth();

        let escrow = Self::load_escrow(&env);
        assert!(
            escrow.status == 2,
            "Escrow must be settled before redemption"
        );

        let mut record = Self::get_investor_record(&env, investor.clone());
        assert!(record.contribution > 0, "No contribution for investor");
        assert!(!record.claimed, "Investor already claimed");

        let (yield_amount, total_payout) =
            Self::compute_expected_payout(&escrow, record.contribution);

        record.claimed = true;
        env.storage()
            .instance()
            .set(&Self::investor_key(investor.clone()), &record);

        // Audit event.
        InvestorRedeemed {
            name: symbol_short!("redeemed"),
            invoice_id: escrow.invoice_id.clone(),
            investor: investor.clone(),
            principal: record.contribution,
            yield_amount,
            total_payout,
        }
        .publish(&env);

        Self::build_position_view(&escrow, &investor, &record)
    }

    fn build_position_view(
        escrow: &InvoiceEscrow,
        investor: &Address,
        record: &InvestorRecord,
    ) -> InvestorPositionView {
        let (yield_amount, total_payout) =
            Self::compute_expected_payout(escrow, record.contribution);
        let claim_status = if record.claimed { 1u32 } else { 0u32 };
        let claimable = escrow.status == 2 && record.contribution > 0 && !record.claimed;
        InvestorPositionView {
            invoice_id: escrow.invoice_id.clone(),
            investor: investor.clone(),
            contribution: record.contribution,
            claim_status,
            claimable,
            expected_principal: record.contribution,
            expected_yield: yield_amount,
            expected_payout: total_payout,
        }
    }

    // ---------------------------------------------------------------------
    // Issue #45: Investor Position Query
    // ---------------------------------------------------------------------

    /// Read-only investor position query for a target escrow.
    ///
    /// Returns the investor's contribution amount, claim status, and the
    /// expected payout details computed from the escrow's yield terms.
    ///
    /// # Security & privacy
    /// - This method is read-only and does not require authorization.
    /// - The returned data contains only public accounting fields already
    ///   represented by on-chain amounts and addresses.
    /// - No off-chain identifiers (KYC data, emails, etc.) are exposed.
    ///
    /// # Panics
    /// - If `target_invoice_id` does not match the escrow stored in this
    ///   contract instance.
    pub fn get_investor_position(
        env: Env,
        target_invoice_id: Symbol,
        investor: Address,
    ) -> InvestorPositionView {
        let escrow = Self::load_escrow(&env);
        assert!(
            escrow.invoice_id == target_invoice_id,
            "Target escrow invoice_id does not match"
        );
        let record = Self::get_investor_record(&env, investor.clone());
        Self::build_position_view(&escrow, &investor, &record)
    }
}

// -------------------------------------------------------------------------
// Tests live in a separate module following Soroban convention.
// -------------------------------------------------------------------------
#[cfg(test)]
mod test;
