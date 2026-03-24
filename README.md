# LiquiFact Contracts

Soroban smart contracts for **LiquiFact** — the global invoice liquidity network on Stellar. This repo contains the **escrow** contract that holds investor funds for tokenized invoices until settlement.

Part of the LiquiFact stack: **frontend** (Next.js) | **backend** (Express) | **contracts** (this repo).

---

## Prerequisites

- **Rust** 1.70+ (stable)
- **Soroban CLI** (optional, for deployment): [Stellar Soroban docs](https://developers.stellar.org/docs/smart-contracts/getting-started/soroban-cli)

For CI and local checks you only need Rust and `cargo`.

---

## Setup

1. **Clone the repo**

   ```bash
   git clone <this-repo-url>
   cd liquifact-contracts
   ```

2. **Build**

   ```bash
   cargo build
   ```

3. **Run tests**

   ```bash
   cargo test
   ```

---

## Development

| Command           | Description                    |
|-------------------|--------------------------------|
| `cargo build`     | Build all contracts            |
| `cargo test`      | Run unit tests                 |
| `cargo fmt`       | Format code                    |
| `cargo fmt -- --check` | Check formatting (used in CI) |

---

## Project structure

```
liquifact-contracts/
├── Cargo.toml           # Workspace definition
├── escrow/
│   ├── Cargo.toml       # Escrow contract crate
│   └── src/
│       ├── lib.rs       # LiquiFact escrow contract (init, fund, settle)
│       └── test.rs      # Unit tests
├── docs/
│   ├── openapi.yaml     # OpenAPI 3.1 specification
│   ├── package.json     # Test runner deps (AJV, js-yaml)
│   └── tests/
│       └── openapi.test.js  # Schema conformance tests (51 cases)
└── .github/workflows/
    └── ci.yml           # CI: fmt, build, test
```

### Escrow contract (high level)

- **init** — Create an invoice escrow (admin, invoice id, SME address, amount, yield bps, maturity). Requires `admin` authorization.
- **get_escrow** — Read current escrow state (no auth required).
- **fund** — Record investor funding; status becomes “funded” when target is met. Requires `investor` authorization.
- **settle** — Mark escrow as settled (buyer paid; investors receive principal + yield). Requires `sme_address` authorization.

### Authorization model

All sensitive state transitions are protected by Soroban's native [`require_auth`](https://developers.stellar.org/docs/smart-contracts/example-contracts/auth) mechanism.

| Function | Required Signer  | Rationale                                                  |
|----------|------------------|------------------------------------------------------------|
| `init`   | `admin`          | Prevents unauthorized escrow creation or re-initialization |
| `fund`   | `investor`       | Each investor authorizes their own contribution            |
| `settle` | `sme_address`    | Only the SME beneficiary may trigger settlement            |

`require_auth` integrates with Soroban's authorization framework: on-chain, the transaction must carry a valid signature (or sub-invocation auth) from the required address. In tests, `env.mock_all_auths()` satisfies all checks so happy-path logic can be verified independently of key management.

#### Security assumptions

- The `admin` address is trusted to create legitimate escrows. Rotate or use a multisig address in production.
- Re-initialization is blocked at the contract level (`"Escrow already initialized"` panic) regardless of who calls `init`.
- `settle` can only move status from `1 → 2`; calling it on an open or already-settled escrow panics.

---

## API documentation (OpenAPI)

The REST API surface is documented in [`docs/openapi.yaml`](docs/openapi.yaml) (OpenAPI 3.1).

### Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/health` | — | Liveness probe |
| `GET` | `/v1/info` | — | API name, version, network |
| `GET` | `/v1/invoices` | JWT | List invoice summaries (paginated) |
| `GET` | `/v1/invoices/{invoiceId}` | JWT | Full escrow detail for one invoice |
| `POST` | `/v1/escrow` | JWT | Initialise a new invoice escrow |
| `POST` | `/v1/escrow/{invoiceId}/fund` | JWT | Record investor funding |
| `POST` | `/v1/escrow/{invoiceId}/settle` | JWT | Settle a funded escrow |

### Security

- All mutating and data endpoints require a `Bearer` JWT in the `Authorization` header.
- `/health` and `/info` are public (no auth required).
- Stellar addresses are validated as 56-char base32 (`[A-Z2-7]`) strings.
- Monetary amounts are always in stroops (smallest unit); `amount ≥ 1` is enforced.
- `yield_bps` is capped at `10000` (100 %) to prevent overflow.

### Running the schema conformance tests

```bash
cd docs
npm install
npm test
# tests 51 | pass 51 | fail 0
```

---

## Execution cost baselines

The escrow contract ships with repeatable cost-measurement tests that track CPU instruction and memory consumption for every core entry point.  These act as regression guards: if a code change causes resource usage to exceed the recorded baseline the test suite fails with a clear message.

### How it works

Soroban's test environment enables **invocation metering** automatically.  After each contract call, `env.cost_estimate().resources()` returns the exact `instructions` and `mem_bytes` consumed by that invocation.  The helper type `CostMeasurement` (defined in `escrow/src/test.rs`) captures these values, prints them, and exposes assertion helpers.

```
[cost] init                           instructions=       33765  mem_bytes=      4602
[cost] fund (partial)                 instructions=       60097  mem_bytes=      9733
[cost] fund (full / status→funded)    instructions=       60097  mem_bytes=      9733
[cost] settle                         instructions=       60093  mem_bytes=      9717
```

Run with output enabled to see the live numbers:

```bash
cargo test test_cost -- --nocapture
```

### Recorded baselines (native build, Soroban SDK 25.x)

| Method | Instructions | Memory (bytes) | Upper-bound limit |
|--------|-------------|----------------|-------------------|
| `init` | ~33 800 | ~4 600 | 100 000 instr / 15 000 mem |
| `fund` (partial) | ~60 100 | ~9 700 | 180 000 instr / 30 000 mem |
| `fund` (full, status flip) | ~60 100 | ~9 700 | 180 000 instr / 30 000 mem |
| `settle` | ~60 100 | ~9 700 | 180 000 instr / 30 000 mem |

Upper bounds are set at **3× the observed baseline** to absorb minor SDK/toolchain drift while still catching significant regressions.

### Profiling methodology and limitations

* Costs are measured against the **native (non-Wasm) contract build** used in tests.  Real on-chain costs include Wasm VM instantiation overhead and will be higher.
* Transaction-size fees and XDR round-trip costs are **not** included.
* For production fee estimates use `stellar-cli contract invoke --simulate` against a live RPC node.
* The upper-bound limits should be **tightened** as the contract stabilises and the Wasm build is benchmarked.

### Updating baselines

If you intentionally change the contract in a way that increases resource usage:

1. Run `cargo test test_cost -- --nocapture` and note the new numbers.
2. Update the `assert_instructions_below` / `assert_mem_below` calls in `escrow/src/test.rs`.
3. Update the table above.
4. Commit with a message explaining why the cost increased.

### Security note

Cost tests do not replace security audits.  They guard against **unintentional** performance regressions.  Always review storage access patterns and authorisation logic separately.

---

## CI/CD

GitHub Actions runs on every push and pull request to `main`:

| Step | Command | Fails if… |
|------|---------|-----------|
| Format | `cargo fmt --all -- --check` | any file is not formatted |
| Build | `cargo build` | compilation error |
| Tests | `cargo test` | any test fails |
| Coverage | `cargo llvm-cov --features testutils --fail-under-lines 95` | line coverage < 95 % |

### Coverage gate

The pipeline uses [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) (installed via `taiki-e/install-action`) to measure line coverage and hard-fail the job when it drops below **95 %**.

To run the coverage check locally:

```bash
# Install once
cargo install cargo-llvm-cov

# Run (requires llvm-tools-preview component)
rustup component add llvm-tools-preview
cargo llvm-cov --features testutils --fail-under-lines 95 --summary-only
```

Keep formatting, tests, and coverage passing before opening a PR.

---

## Contributing

1. **Fork** the repo and clone your fork.
2. **Create a branch** from `main`: `git checkout -b feature/your-feature` or `fix/your-fix`.
3. **Setup**: ensure Rust stable is installed; run `cargo build` and `cargo test`.
4. **Make changes**:
   - Follow existing patterns in `escrow/src/lib.rs`.
   - Add or update tests in `escrow/src/test.rs`.
   - Format with `cargo fmt`.
5. **Verify locally**:
   - `cargo fmt --all -- --check`
   - `cargo build`
   - `cargo test`
6. **Commit** with clear messages (e.g. `feat(escrow): X`, `test(escrow): Y`).
7. **Push** to your fork and open a **Pull Request** to `main`.
8. Wait for CI and address review feedback.

We welcome new contracts (e.g. settlement, tokenization helpers), tests, and docs that align with LiquiFact’s invoice financing flow.

---

## License

MIT (see root LiquiFact project for full license).
