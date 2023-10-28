use std::num::NonZeroU32;

use axum::extract::{Json, Path, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use serde::{Deserialize, Serialize};

use super::middleware::auth::UserUuid;
use super::InternalApiState;
use crate::trading::{
    OrderSide, OrderType, OrderUuid, PlaceOrder, PlaceOrderResult, SelfTradeProtection,
    TimeInForce, TradingEngineError as TErr,
};
use crate::Asset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAddOrder {
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: NonZeroU32,
    pub price: NonZeroU32,
    #[serde(default)]
    pub time_in_force: TimeInForce,
    #[serde(default)]
    pub stp: SelfTradeProtection,
}

#[derive(Debug, Serialize)]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

/// Place an order for `asset`
pub async fn trade_add_order(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_uuid)): Extension<UserUuid>,
    Path(asset): Path<String>,
    Json(body): Json<TradeAddOrder>,
) -> Response {
    let asset = match asset.as_str() {
        "btc" | "BTC" => Asset::Bitcoin,
        "eth" | "ETH" => Asset::Ether,
        _ => {
            tracing::warn!(?asset, "invalid asset");
            return (axum::http::StatusCode::NOT_FOUND, "invalid asset").into_response();
        }
    };

    if !state
        .assets
        .contains_key(&crate::asset::AssetKey::ByValue(asset))
    {
        tracing::warn!(?asset, "asset not enabled");
        return (axum::http::StatusCode::NOT_FOUND, "asset not enabled").into_response();
    } else {
        tracing::info!(?asset, "placing order for asset");
    }

    let (response, reserved_funds) = match state.app_cx.place_order(asset, user_uuid, body).await {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(?err, "failed to place order");
            return super::internal_server_error("failed to place order");
        }
    };

    let _deferred_revert = reserved_funds.defer_revert(
        tokio::runtime::Handle::current(),
        state.app_cx.db_pool.clone(),
    );

    let order_uuid = response.wait().await;

    if matches!(order_uuid, Some(Ok(_))) {
        _deferred_revert.cancel();
    }

    match order_uuid {
        Some(Ok(PlaceOrderResult { order_uuid, .. })) => {
            tracing::info!(?order_uuid, "order placed");
            Json(TradeAddOrderResponse {
                order_uuid: order_uuid.0,
            })
            .into_response()
        }
        Some(Err(err)) => match err {
            TErr::UnserializableInput => super::internal_server_error(
                "this input was considered problematic and could not be processed",
            ),
            err => {
                tracing::warn!(?err, "failed to place order");
                super::internal_server_error("failed to place order")
            }
        },
        None => {
            tracing::warn!("trading engine unresponsive");
            super::internal_server_error("trading engine unresponsive")
        }
    }
}
