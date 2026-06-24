use crate::{Error, HyperliquidClient};
use serde_json::{Value, json};

impl HyperliquidClient {
    pub async fn meta(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "meta" })).await
    }

    pub async fn meta_for_dex(&self, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "meta", "dex": dex })).await
    }

    pub async fn meta_and_asset_ctxs(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "metaAndAssetCtxs" })).await
    }

    pub async fn meta_and_asset_ctxs_for_dex(&self, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "metaAndAssetCtxs", "dex": dex }))
            .await
    }

    pub async fn perp_dexs(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpDexs" })).await
    }

    pub async fn spot_meta(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "spotMeta" })).await
    }

    pub async fn spot_meta_and_asset_ctxs(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "spotMetaAndAssetCtxs" }))
            .await
    }

    pub async fn spot_deploy_state(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "spotDeployState", "user": user }))
            .await
    }

    pub async fn spot_pair_deploy_auction_status(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "spotPairDeployAuctionStatus" }))
            .await
    }

    pub async fn token_details(&self, token_id: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "tokenDetails", "tokenId": token_id }))
            .await
    }

    pub async fn outcome_meta(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "outcomeMeta" })).await
    }

    pub async fn settled_outcome(&self, outcome: u64) -> Result<Value, Error> {
        self.send_info(json!({ "type": "settledOutcome", "outcome": outcome }))
            .await
    }

    pub async fn all_mids(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "allMids" })).await
    }

    pub async fn all_mids_for_dex(&self, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "allMids", "dex": dex }))
            .await
    }

    pub async fn l2_book(&self, coin: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "l2Book", "coin": coin }))
            .await
    }

    pub async fn l2_book_with_precision(
        &self,
        coin: &str,
        n_sig_figs: u32,
        mantissa: Option<u32>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "l2Book",
            "coin": coin,
            "nSigFigs": n_sig_figs
        });
        if let Some(mantissa) = mantissa {
            body["mantissa"] = json!(mantissa);
        }
        self.send_info(body).await
    }

    pub async fn candle_snapshot(
        &self,
        coin: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Value, Error> {
        self.send_info(json!({
            "type": "candleSnapshot",
            "req": {
                "coin": coin,
                "interval": interval,
                "startTime": start_time,
                "endTime": end_time
            }
        }))
        .await
    }

    pub async fn funding_history(
        &self,
        coin: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "fundingHistory",
            "coin": coin,
            "startTime": start_time
        });
        if let Some(end_time) = end_time {
            body["endTime"] = json!(end_time);
        }
        self.send_info(body).await
    }

    pub async fn clearinghouse_state(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "clearinghouseState", "user": user }))
            .await
    }

    pub async fn clearinghouse_state_for_dex(&self, user: &str, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "clearinghouseState", "user": user, "dex": dex }))
            .await
    }

    pub async fn spot_clearinghouse_state(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "spotClearinghouseState", "user": user }))
            .await
    }

    pub async fn user_non_funding_ledger_updates(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "userNonFundingLedgerUpdates",
            "user": user,
            "startTime": start_time
        });
        if let Some(end_time) = end_time {
            body["endTime"] = json!(end_time);
        }
        self.send_info(body).await
    }

    pub async fn user_funding(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "userFunding",
            "user": user,
            "startTime": start_time
        });
        if let Some(end_time) = end_time {
            body["endTime"] = json!(end_time);
        }
        self.send_info(body).await
    }

    pub async fn predicted_fundings(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "predictedFundings" })).await
    }

    pub async fn perps_at_open_interest_cap(&self, dex: Option<&str>) -> Result<Value, Error> {
        let mut body = json!({ "type": "perpsAtOpenInterestCap" });
        if let Some(dex) = dex {
            body["dex"] = json!(dex);
        }
        self.send_info(body).await
    }

    pub async fn perp_deploy_auction_status(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpDeployAuctionStatus" }))
            .await
    }

    pub async fn active_asset_data(&self, user: &str, coin: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "activeAssetData", "user": user, "coin": coin }))
            .await
    }

    pub async fn perp_dex_limits(&self, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpDexLimits", "dex": dex }))
            .await
    }

    pub async fn perp_dex_status(&self, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpDexStatus", "dex": dex }))
            .await
    }

    pub async fn all_perp_metas(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "allPerpMetas" })).await
    }

    pub async fn perp_annotation(&self, coin: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpAnnotation", "coin": coin }))
            .await
    }

    pub async fn perp_categories(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpCategories" })).await
    }

    pub async fn perp_concise_annotations(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "perpConciseAnnotations" }))
            .await
    }

    pub async fn user_rate_limit(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userRateLimit", "user": user }))
            .await
    }

    pub async fn user_role(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userRole", "user": user }))
            .await
    }

    pub async fn user_fees(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userFees", "user": user }))
            .await
    }

    pub async fn referral(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "referral", "user": user }))
            .await
    }

    pub async fn delegations(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "delegations", "user": user }))
            .await
    }

    pub async fn delegator_summary(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "delegatorSummary", "user": user }))
            .await
    }

    pub async fn delegator_history(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "delegatorHistory", "user": user }))
            .await
    }

    pub async fn delegator_rewards(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "delegatorRewards", "user": user }))
            .await
    }

    pub async fn sub_accounts(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "subAccounts", "user": user }))
            .await
    }

    pub async fn portfolio(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "portfolio", "user": user }))
            .await
    }

    pub async fn vault_details(
        &self,
        vault_address: &str,
        user: Option<&str>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "vaultDetails",
            "vaultAddress": vault_address
        });
        if let Some(user) = user {
            body["user"] = json!(user);
        }
        self.send_info(body).await
    }

    pub async fn user_vault_equities(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userVaultEquities", "user": user }))
            .await
    }

    pub async fn user_abstraction(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userAbstraction", "user": user }))
            .await
    }

    pub async fn user_dex_abstraction(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userDexAbstraction", "user": user }))
            .await
    }

    pub async fn borrow_lend_user_state(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "borrowLendUserState", "user": user }))
            .await
    }

    pub async fn borrow_lend_reserve_state(&self, token: u64) -> Result<Value, Error> {
        self.send_info(json!({ "type": "borrowLendReserveState", "token": token }))
            .await
    }

    pub async fn all_borrow_lend_reserve_states(&self) -> Result<Value, Error> {
        self.send_info(json!({ "type": "allBorrowLendReserveStates" }))
            .await
    }

    pub async fn approved_builders(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "approvedBuilders", "user": user }))
            .await
    }

    pub async fn max_builder_fee(&self, user: &str, builder: &str) -> Result<Value, Error> {
        self.send_info(json!({
            "type": "maxBuilderFee",
            "user": user,
            "builder": builder
        }))
        .await
    }

    pub async fn open_orders(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "openOrders", "user": user }))
            .await
    }

    pub async fn open_orders_for_dex(&self, user: &str, dex: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "openOrders", "user": user, "dex": dex }))
            .await
    }

    pub async fn frontend_open_orders(
        &self,
        user: &str,
        dex: Option<&str>,
    ) -> Result<Value, Error> {
        let mut body = json!({ "type": "frontendOpenOrders", "user": user });
        if let Some(dex) = dex {
            body["dex"] = json!(dex);
        }
        self.send_info(body).await
    }

    pub async fn order_status(&self, user: &str, oid: &str) -> Result<Value, Error> {
        let oid = oid
            .parse::<u64>()
            .map(Value::from)
            .unwrap_or_else(|_| json!(oid));
        self.send_info(json!({ "type": "orderStatus", "user": user, "oid": oid }))
            .await
    }

    pub async fn historical_orders(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "historicalOrders", "user": user }))
            .await
    }

    pub async fn user_twap_slice_fills(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userTwapSliceFills", "user": user }))
            .await
    }

    pub async fn user_fills(&self, user: &str) -> Result<Value, Error> {
        self.send_info(json!({ "type": "userFills", "user": user }))
            .await
    }

    pub async fn user_fills_aggregated(
        &self,
        user: &str,
        aggregate_by_time: bool,
    ) -> Result<Value, Error> {
        self.send_info(json!({
            "type": "userFills",
            "user": user,
            "aggregateByTime": aggregate_by_time
        }))
        .await
    }

    pub async fn user_fills_by_time(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "userFillsByTime",
            "user": user,
            "startTime": start_time
        });
        if let Some(end_time) = end_time {
            body["endTime"] = json!(end_time);
        }
        self.send_info(body).await
    }

    pub async fn user_fills_by_time_aggregated(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
        aggregate_by_time: bool,
    ) -> Result<Value, Error> {
        let mut body = json!({
            "type": "userFillsByTime",
            "user": user,
            "startTime": start_time,
            "aggregateByTime": aggregate_by_time
        });
        if let Some(end_time) = end_time {
            body["endTime"] = json!(end_time);
        }
        self.send_info(body).await
    }
}
