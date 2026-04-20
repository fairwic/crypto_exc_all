# Binance Futures SDK Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first usable `binance_rs` SDK slice for Binance USDⓈ-M Futures, including signed account balance requests using `.env` API credentials.

**Architecture:** Create a standalone `binance_rs` crate that mirrors the existing OKX crate shape: config, client, API domain modules, DTOs, errors, and utilities. The client owns request signing, headers, response parsing, and base URL configuration so later API modules stay thin.

**Tech Stack:** Rust 2024, Tokio, Reqwest with rustls, Serde, thiserror, dotenv, HMAC-SHA256, mockito.

### Task 1: Crate Skeleton and Dependencies

**Files:**
- Create: `binance_rs/Cargo.toml`
- Create: `binance_rs/src/lib.rs`
- Create: `binance_rs/src/api/mod.rs`
- Create: `binance_rs/src/api/api_trait.rs`
- Create: `binance_rs/src/dto/mod.rs`

**Step 1: Write minimal crate files**

Create the crate with dependencies for async HTTP, JSON DTOs, dotenv config, HMAC signing, and mock testing.

**Step 2: Run metadata check**

Run: `cargo metadata --manifest-path binance_rs/Cargo.toml --no-deps --format-version 1`

Expected: metadata succeeds and reports package `binance_rs`.

### Task 2: Config, Errors, and Signing

**Files:**
- Create: `binance_rs/src/config.rs`
- Create: `binance_rs/src/error.rs`
- Create: `binance_rs/src/utils.rs`

**Step 1: Write failing tests**

Add tests for:
- Binance documentation HMAC example payload signs to `3c661234138461fcc7a7d8746c6558c9842d4e10870d2ecbedf7777cad694af9`.
- Query serialization keeps deterministic order and adds no trailing separator.

**Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path binance_rs/Cargo.toml utils -- --nocapture`

Expected: failure before implementation.

**Step 3: Implement minimal code**

Implement:
- `Credentials::from_env()` with `BINANCE_API_KEY` / `binance_api_key` and `BINANCE_API_SECRET` / `binance_api_secret` fallback.
- `Config::from_env()` with default `https://fapi.binance.com`, timeout, and recvWindow 5000.
- `generate_signature(secret, payload) -> Result<String, Error>`.
- `build_query_string(&[(impl AsRef<str>, impl AsRef<str>)])`.

**Step 4: Run tests**

Run: `cargo test --manifest-path binance_rs/Cargo.toml utils -- --nocapture`

Expected: pass.

### Task 3: HTTP Client

**Files:**
- Create: `binance_rs/src/client.rs`

**Step 1: Write failing tests**

Use mockito to verify a signed `GET /fapi/v2/balance` request sends:
- Header `X-MBX-APIKEY`.
- Query params `recvWindow`, `timestamp`, and `signature`.
- Parses the response body into the requested DTO.

Add another mock test for non-2xx Binance error payload `{ "code": -2015, "msg": "Invalid API-key..." }`.

**Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path binance_rs/Cargo.toml client -- --nocapture`

Expected: failure before implementation.

**Step 3: Implement minimal code**

Implement:
- `BinanceClient::new(credentials)`.
- `BinanceClient::new_public()`.
- `BinanceClient::from_env()`.
- `set_base_url` for tests.
- `send_public_request<T>()`.
- `send_signed_request<T>()`, signing query-string params and appending `signature`.
- Binance error parsing and status-aware errors.

**Step 4: Run tests**

Run: `cargo test --manifest-path binance_rs/Cargo.toml client -- --nocapture`

Expected: pass.

### Task 4: First API Feature

**Files:**
- Create: `binance_rs/src/api/account/account_api.rs`
- Create: `binance_rs/src/api/account/mod.rs`
- Create: `binance_rs/src/api/market/market_api.rs`
- Create: `binance_rs/src/api/market/mod.rs`
- Create: `binance_rs/src/dto/account/account_dto.rs`
- Create: `binance_rs/src/dto/account/mod.rs`
- Create: `binance_rs/src/dto/market/market_dto.rs`
- Create: `binance_rs/src/dto/market/mod.rs`
- Modify: `binance_rs/src/api/mod.rs`
- Modify: `binance_rs/src/dto/mod.rs`
- Modify: `binance_rs/src/lib.rs`

**Step 1: Write tests**

Add mock tests for:
- `BinanceAccount::get_balance()` maps `GET /fapi/v2/balance`.
- `BinanceMarket::get_server_time()` maps `GET /fapi/v1/time`.

**Step 2: Implement API wrappers**

Implement:
- `BinanceAccount::from_env()` and `get_balance() -> Result<Vec<AccountBalance>, Error>`.
- `BinanceMarket::new_public()` and `get_server_time() -> Result<ServerTime, Error>`.

**Step 3: Run tests**

Run: `cargo test --manifest-path binance_rs/Cargo.toml -- --nocapture`

Expected: pass.

### Task 5: Documentation and Live Read-Only Check

**Files:**
- Create: `binance_rs/README.md`
- Optionally create: `binance_rs/examples/account_balance.rs`

**Step 1: Document use**

Document `.env` variables and the first supported endpoints. Do not print secrets.

**Step 2: Run verification**

Run:
- `cargo fmt --manifest-path binance_rs/Cargo.toml`
- `cargo test --manifest-path binance_rs/Cargo.toml -- --nocapture`
- A read-only live request if credentials/network allow it; never place orders.

Expected: formatting succeeds, tests pass, live check either returns a safe read-only response or records the external API/auth failure.

### Task 6: Expanded Binance REST Surface

**Files:**
- Modify: `binance_rs/src/api/market/market_api.rs`
- Modify: `binance_rs/src/api/account/account_api.rs`
- Create: `binance_rs/src/api/trade/trade_api.rs`
- Create: `binance_rs/src/api/trade/mod.rs`
- Modify: `binance_rs/src/api/mod.rs`
- Modify: `binance_rs/src/lib.rs`
- Test: `binance_rs/tests/expanded_api_tests.rs`

**Implemented endpoints:**
- Market: `exchangeInfo`, `depth`, `klines`, `ticker/24hr`, `fundingRate`.
- Account: `account`, `positionRisk`, `income`.
- Trade: `order`, `order/test`, `openOrders`, `allOrders`, `userTrades`, `leverage`.

**Verification:**
- Expanded API mock tests verify HTTP method, path, query params, auth header, timestamp, recvWindow, and signatures for representative signed calls.
