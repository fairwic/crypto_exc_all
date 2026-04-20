# Bitget SDK Aggregation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Bitget as a first-class exchange in `crypto_exc_all` with an independent `bitget_rs` crate and root facade aggregation.

**Architecture:** `bitget_rs` owns Bitget V2 REST signing, request dispatch, DTOs, and mock tests. The root crate owns only configuration discovery, `ExchangeId` registration, symbol mapping, error conversion, and unified `MarketFacade` / `AccountFacade` adapters.

**Tech Stack:** Rust 2024, Tokio async tests, Reqwest, Serde, HMAC-SHA256 + Base64 signing, mockito.

### Task 1: Root Bitget Behavior Tests

**Files:**
- Modify: `tests/external_consumer_tests.rs`
- Modify: `src/config.rs` test module
- Modify: `src/sdk.rs` test module

**Steps:**
1. Extend the external consumer test to configure a Bitget mock server.
2. Assert `sdk.market(ExchangeId::Bitget).ticker(&Instrument::perp("BTC", "USDT"))` calls `/api/v2/mix/market/ticker?productType=USDT-FUTURES&symbol=BTCUSDT`.
3. Add config test coverage for `BITGET_API_KEY`, `BITGET_API_SECRET`, `BITGET_PASSPHRASE`, proxy normalization, and product type.
4. Add SDK construction expectations for configured exchanges sorted by exchange id.
5. Run the targeted tests and confirm they fail because root Bitget support is not implemented.

### Task 2: Root Facade Implementation

**Files:**
- Modify: `src/exchange.rs`
- Modify: `src/config.rs`
- Modify: `src/error.rs`
- Modify: `src/instrument.rs`
- Modify: `src/adapters/mod.rs`
- Create: `src/adapters/bitget.rs`
- Modify: `src/sdk.rs`
- Modify: `src/lib.rs`

**Steps:**
1. Add `ExchangeId::Bitget` parse/display/serde support.
2. Add `BitgetExchangeConfig` and environment loading.
3. Add `Instrument` mapping to Bitget's `BASEQUOTE` futures symbols.
4. Convert `bitget_rs::Error` into root `Error`.
5. Add `BitgetAdapter` mapping native ticker/account DTOs into root `Ticker` and `Balance`.
6. Register Bitget in `CryptoSdk::from_config`.
7. Re-run root tests and fix only issues required by the failing tests.

### Task 3: Documentation and Packaging

**Files:**
- Create: `bitget_rs/README.md`
- Modify: `README.md`
- Modify: `docs/exchange-integration-playbook.md`

**Steps:**
1. Document Bitget environment variables and current V2 Futures coverage.
2. Add Bitget to the monorepo publishing structure and root crate feature list.
3. Keep the playbook updated so the next exchange can reuse the same checklist.

### Task 4: Verification

**Commands:**
- `cargo fmt --all`
- `cargo test -p bitget_rs -- --nocapture`
- `cargo test -p crypto_exc_all -- --nocapture`
- `cargo check --workspace`
- `cargo clippy -p crypto_exc_all --all-targets --no-deps -- -D warnings`

**Expected Result:** All commands exit with code 0. Any failure must be debugged with `superpowers:systematic-debugging` before changing code.

### Task 5: REST Parity Expansion

**Files:**
- Modify: `bitget_rs/src/api/market.rs`
- Modify: `bitget_rs/src/api/account.rs`
- Modify: `bitget_rs/src/api/trade.rs`
- Modify: `bitget_rs/src/api/asset.rs`
- Create: `bitget_rs/src/api/announcements.rs`
- Modify: `bitget_rs/tests/api_tests.rs`
- Modify: `README.md`, `bitget_rs/README.md`, `notes.md`, `task_plan.md`

**Steps:**
1. Add mocked RED tests for Bitget equivalents of OKX/Binance market/account/trade/asset/announcement REST domains.
2. Implement public V2 market helpers for server time, contract config, orderbook, candles, funding, OI, long/short ratios, taker buy/sell, and exchange rate.
3. Implement signed account helpers for account list, single account, account bills, positions, leverage, margin mode, position mode, isolated margin adjustment, asset mode, and trade rate.
4. Implement signed trade helpers for place/cancel/batch/modify/cancel-all/close-position plus order detail, pending orders, order history, and fills.
5. Implement spot wallet/asset helpers for coin info, deposit address/history, withdrawal history, transfer coin info, transfer, and withdrawal.
6. Document the new REST coverage and remaining WebSocket parity gap.
