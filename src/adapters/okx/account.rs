use super::*;

impl OkxAdapter {
    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        Ok(self
            .sourced_balances()
            .await?
            .into_iter()
            .map(|value| value.balance)
            .collect())
    }

    /// 保留 OKX account-level `uTime`，不把本地响应时间冒充 provider source time。
    pub(crate) async fn sourced_balances(&self) -> Result<Vec<SourcedBalance>> {
        let accounts = self
            .account
            .get_balance(None)
            .await
            .map_err(Error::from_okx)?;
        let mut output = Vec::new();

        for account in accounts {
            let source_updated_at_ms =
                parse_u64_string(&account.u_time).ok_or_else(|| Error::Adapter {
                    exchange: ExchangeId::Okx,
                    message: "OKX balance response missing uTime".to_owned(),
                })?;
            for detail in account.details {
                let raw = serde_json::to_value(&detail)?;
                output.push(SourcedBalance {
                    balance: Balance {
                        exchange: ExchangeId::Okx,
                        asset: detail.ccy,
                        total: detail.eq,
                        available: if detail.avail_bal.is_empty() {
                            detail.avail_eq
                        } else {
                            detail.avail_bal
                        },
                        frozen: non_empty(detail.frozen_bal),
                        raw,
                    },
                    source_updated_at_ms,
                });
            }
        }

        Ok(output)
    }

    /// 读取 OKX 币种明细的权益、可用保证金与占用保证金。
    pub(crate) async fn margin_summary(
        &self,
        quote_currency: &str,
    ) -> Result<AccountMarginSummary> {
        let accounts = self
            .account
            .get_balance(Some(quote_currency))
            .await
            .map_err(Error::from_okx)?;
        margin_summary::map_margin_summary(accounts, quote_currency)
    }

    /// 映射 OKX `/api/v5/account/config` 当前官方 identity 与账户模式字段。
    pub(crate) async fn account_identity(&self) -> Result<AccountIdentity> {
        let exchange = ExchangeId::Okx;
        let raw = self
            .account
            .get_config_raw()
            .await
            .map_err(Error::from_okx)?;
        let object = raw
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_object)
            .or_else(|| raw.as_object())
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX account config response missing object".to_owned(),
            })?;
        let provider_account_id = string_field(object, "uid")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX account config response missing uid".to_owned(),
            })?;
        let account_level = string_field(object, "acctLv").unwrap_or_default();
        let margin_mode = okx_account_mode(&account_level).ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX account config returned an unknown acctLv".to_owned(),
        })?;
        let position_mode = string_field(object, "posMode")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "OKX account config response missing posMode".to_owned(),
            })?;
        let parent_account_id = okx_parent_account_id(
            &provider_account_id,
            string_field(object, "mainUid").filter(|value| !value.trim().is_empty()),
        );
        Ok(AccountIdentity {
            exchange,
            provider_account_id,
            parent_account_id,
            margin_mode: margin_mode.to_owned(),
            position_mode,
            settlement_asset: "USDT".to_owned(),
        })
    }

    /// 复用 OKX signed account config，只映射官方 `perm` 权限集合。
    pub(crate) async fn account_order_permission(&self) -> Result<AccountOrderPermission> {
        let raw = self
            .account
            .get_config_raw()
            .await
            .map_err(Error::from_okx)?;
        account_permission::map_account_order_permission(raw)
    }

    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        let exchange = ExchangeId::Okx;
        let symbol_hint = query
            .instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let inst_type = query
            .inst_type
            .clone()
            .or_else(|| query.instrument.as_ref().map(okx_inst_type_for_instrument));
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = if query.archive {
            self.account
                .get_bills_archive(
                    inst_type.as_deref(),
                    symbol_hint.as_deref(),
                    query.asset.as_deref(),
                    None,
                    query.bill_type.as_deref(),
                    begin.as_deref(),
                    end.as_deref(),
                    query.limit,
                )
                .await
        } else {
            self.account
                .get_bills(
                    inst_type.as_deref(),
                    symbol_hint.as_deref(),
                    query.asset.as_deref(),
                    None,
                    query.bill_type.as_deref(),
                    begin.as_deref(),
                    end.as_deref(),
                    query.limit,
                )
                .await
        }
        .map_err(Error::from_okx)?;

        okx_account_bills_from_value(exchange, query.instrument, symbol_hint, raw)
    }

    /// 调用 OKX account max-size，返回交易所原始下单单位的买/卖最大数量。
    pub(crate) async fn max_order_size(
        &self,
        request: MaxOrderSizeRequest,
    ) -> Result<MaxOrderSize> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let td_mode = request.margin_mode.as_okx_td_mode();
        let response = self
            .account
            .get_max_size(
                &symbol,
                &td_mode,
                request.margin_coin.as_deref(),
                request.price.as_deref(),
                request.leverage.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;
        let item = response
            .into_iter()
            .find(|item| item.inst_id.eq_ignore_ascii_case(&symbol))
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("OKX max-size response is empty for {symbol}"),
            })?;

        okx_max_order_size_from_dto(exchange, instrument, symbol, request, item)
    }

    pub(crate) async fn set_leverage(
        &self,
        request: SetLeverageRequest,
    ) -> Result<LeverageSetting> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let ccy = if matches!(
            &instrument.market_type,
            MarketType::Spot | MarketType::Margin
        ) {
            request.margin_coin.clone()
        } else {
            None
        };
        let raw = self
            .account
            .set_leverage(OkxSetLeverageRequest {
                inst_id: Some(symbol.clone()),
                ccy,
                lever: request.leverage.clone(),
                mgn_mode: okx_margin_mode(request.margin_mode.as_ref()),
                pos_side: request
                    .position_side
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase()),
            })
            .await
            .map_err(Error::from_okx)?;

        okx_leverage_setting_from_value(exchange, instrument, symbol, request, raw)
    }

    pub(crate) async fn leverage_info(
        &self,
        query: LeverageInfoQuery,
    ) -> Result<Vec<LeverageSetting>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let margin_mode = query.margin_mode.as_okx_td_mode();
        let raw = self
            .account
            .get_leverage_info(&symbol, &margin_mode)
            .await
            .map_err(Error::from_okx)?;
        okx_leverage_info_from_value(exchange, instrument, symbol, &margin_mode, raw)
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: true,
            set_position_mode: true,
            set_symbol_margin_mode: false,
            order_level_margin_mode: true,
        }
    }

    pub(crate) async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        let exchange = ExchangeId::Okx;
        let raw_mode = okx_position_mode(request.mode);
        if let Some(current_mode) = self.current_position_mode().await?
            && current_mode.eq_ignore_ascii_case(raw_mode)
        {
            return Ok(PositionModeSetting {
                exchange,
                mode: request.mode,
                raw_mode: Some(current_mode),
                product_type: request.product_type,
                raw: json!({
                    "posMode": raw_mode,
                    "idempotent": true,
                    "source": "account/config",
                }),
            });
        }
        let raw = self
            .account
            .set_position_mode(OkxSetPositionModeRequest {
                pos_mode: raw_mode.to_string(),
            })
            .await
            .map_err(Error::from_okx)?;

        okx_position_mode_setting_from_value(exchange, request, raw)
    }

    /// 只读读取 OKX 当前仓位模式，避免对已匹配账户重复调用 set-position-mode mutation。
    async fn current_position_mode(&self) -> Result<Option<String>> {
        let exchange = ExchangeId::Okx;
        let raw = self
            .account
            .get_config_raw()
            .await
            .map_err(Error::from_okx)?;
        okx_position_mode_from_config(raw, exchange)
    }

    pub(crate) async fn set_symbol_margin_mode(
        &self,
        _request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        Err(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "set_symbol_margin_mode",
        })
    }

    pub(crate) async fn ensure_order_margin_mode(
        &self,
        request: EnsureOrderMarginModeRequest,
    ) -> Result<EnsureOrderMarginModeResult> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument;
        let exchange_symbol = instrument.symbol_for(exchange);
        let raw_mode = request.mode.as_okx_td_mode();

        Ok(EnsureOrderMarginModeResult {
            exchange,
            instrument,
            exchange_symbol,
            mode: request.mode,
            apply_method: MarginModeApplyMethod::OrderLevel,
            raw_mode: Some(raw_mode),
            product_type: request.product_type,
            margin_coin: request.margin_coin,
            raw: Value::Null,
        })
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        Ok(self
            .position_rows(instrument)
            .await?
            .into_iter()
            .map(|(position, _)| position)
            .collect())
    }

    /// 保留 OKX position `uTime`，供 Account snapshot 与私有流按同一时钟合并。
    pub(crate) async fn sourced_positions(
        &self,
        instrument: Option<&Instrument>,
    ) -> Result<Vec<SourcedPosition>> {
        self.position_rows(instrument)
            .await?
            .into_iter()
            .map(|(position, source_updated_at_ms)| {
                Ok(SourcedPosition {
                    position,
                    source_updated_at_ms: source_updated_at_ms.ok_or_else(|| Error::Adapter {
                        exchange: ExchangeId::Okx,
                        message: "OKX position response missing uTime".to_owned(),
                    })?,
                })
            })
            .collect()
    }

    /// 共享一次 DTO 映射，同时让旧接口继续接受没有 `uTime` 的历史响应。
    async fn position_rows(
        &self,
        instrument: Option<&Instrument>,
    ) -> Result<Vec<(Position, Option<u64>)>> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let positions = self
            .account
            .get_positions(Some("SWAP"), symbol.as_deref(), None)
            .await
            .map_err(Error::from_okx)?;

        positions
            .into_iter()
            .map(|position| {
                let source_updated_at_ms =
                    position.update_time.as_deref().and_then(parse_u64_string);
                let raw = serde_json::to_value(&position)?;
                let mapped_instrument = instrument
                    .cloned()
                    .unwrap_or_else(|| instrument_from_okx_symbol(&position.inst_id));
                Ok((
                    Position {
                        exchange,
                        instrument: mapped_instrument,
                        exchange_symbol: position.inst_id,
                        side: Some(position.position_side.as_str().to_string()),
                        size: position.pos,
                        entry_price: non_empty(position.average_price),
                        mark_price: None,
                        unrealized_pnl: non_empty(position.upl),
                        leverage: non_empty(position.leverage),
                        margin_mode: Some(
                            okx_margin_mode_from_enum(position.margin_mode).to_string(),
                        ),
                        liquidation_price: position.liquidation_price.and_then(non_empty),
                        raw,
                    },
                    source_updated_at_ms,
                ))
            })
            .collect()
    }

    pub(crate) async fn position_history(
        &self,
        query: PositionHistoryQuery,
    ) -> Result<Vec<PositionHistory>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let inst_type = query
            .instrument_type
            .clone()
            .or_else(|| instrument.as_ref().map(okx_inst_type_for_instrument))
            .unwrap_or_else(|| "SWAP".to_string());
        let response = self
            .account
            .get_positions_history(
                Some(inst_type.as_str()),
                symbol.as_deref(),
                query.margin_mode.as_deref(),
                query.close_type.as_deref(),
                query.position_id.as_deref(),
                query.after.as_deref(),
                query.before.as_deref(),
                query.limit,
            )
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|item| {
                let value = serde_json::to_value(item)?;
                okx_position_history_from_value(exchange, instrument.clone(), symbol.clone(), value)
            })
            .collect()
    }
}
