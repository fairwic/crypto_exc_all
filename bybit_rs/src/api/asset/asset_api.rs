use crate::client::to_params;
use crate::{
    BybitClient, BybitDepositRecordRequest, BybitTransferRecordRequest,
    BybitWithdrawalRecordRequest, Error,
};
use serde_json::Value;

impl BybitClient {
    pub async fn internal_transfer_records(
        &self,
        request: BybitTransferRecordRequest,
    ) -> Result<Value, Error> {
        let params = to_params(&request)?;
        self.send_signed_get("/v5/asset/transfer/query-inter-transfer-list", &params)
            .await
    }

    pub async fn deposit_records(
        &self,
        request: BybitDepositRecordRequest,
    ) -> Result<Value, Error> {
        let params = to_params(&request)?;
        self.send_signed_get("/v5/asset/deposit/query-record", &params)
            .await
    }

    pub async fn withdrawal_records(
        &self,
        request: BybitWithdrawalRecordRequest,
    ) -> Result<Value, Error> {
        let params = to_params(&request)?;
        self.send_signed_get("/v5/asset/withdraw/query-record", &params)
            .await
    }
}
