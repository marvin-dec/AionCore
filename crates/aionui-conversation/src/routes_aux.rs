#![allow(clippy::disallowed_types)]

use crate::state::ConversationRouterState;
use aionui_api_types::{
    ApiResponse, SetConfigOptionRequest, SetConfigOptionResponse, SideQuestionRequest, SideQuestionResponse,
    SlashCommandItem, WorkspaceBrowseQuery, WorkspaceEntry,
};
use aionui_auth::CurrentUser;
use aionui_common::ApiError;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Json, Path, Query, State};
use axum::routing::{get, post, put};

/// Build the conversation-ops router (no auth layer applied — the caller is
/// responsible for wrapping this with the auth middleware).
pub fn conversation_ops_routes(state: ConversationRouterState) -> Router {
    Router::new()
        .route("/api/conversations/{id}/side-question", post(side_question))
        .route("/api/conversations/{id}/slash-commands", get(get_slash_commands))
        .route("/api/conversations/{id}/usage", get(get_usage))
        .route(
            "/api/conversations/{id}/config-options/{option_id}",
            put(set_config_option),
        )
        .route("/api/conversations/{id}/workspace", get(browse_workspace))
        .route("/api/conversations/{id}/ext-method", post(ext_method))
        .with_state(state)
}

// ── Route handlers ─────────────────────────────────────────────────

async fn set_config_option(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path((id, option_id)): Path<(String, String)>,
    body: Result<Json<SetConfigOptionRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<SetConfigOptionResponse>>, ApiError> {
    let Json(req) = body.map_err(ApiError::from)?;
    Ok(Json(ApiResponse::ok(
        state
            .service
            .set_config_option(&user.id, &id, &option_id, req)
            .await
            .map_err(ApiError::from)?,
    )))
}

async fn get_usage(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Option<serde_json::Value>>>, ApiError> {
    Ok(Json(ApiResponse::ok(
        state.service.get_usage(&user.id, &id).await.map_err(ApiError::from)?,
    )))
}

async fn side_question(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(req): Json<SideQuestionRequest>,
) -> Result<Json<ApiResponse<SideQuestionResponse>>, ApiError> {
    Ok(Json(ApiResponse::ok(
        state
            .service
            .handle_side_question(&user.id, &id, req)
            .await
            .map_err(ApiError::from)?,
    )))
}

async fn get_slash_commands(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<SlashCommandItem>>>, ApiError> {
    Ok(Json(ApiResponse::ok(
        state
            .service
            .get_slash_commands(&user.id, &id)
            .await
            .map_err(ApiError::from)?,
    )))
}

async fn browse_workspace(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Query(query): Query<WorkspaceBrowseQuery>,
) -> Result<Json<ApiResponse<Vec<WorkspaceEntry>>>, ApiError> {
    Ok(Json(ApiResponse::ok(
        state
            .service
            .browse_workspace(&user.id, &id, query)
            .await
            .map_err(ApiError::from)?,
    )))
}

// ── ACP extMethod 转发 ──────────────────────────────────────────────

/// 请求体：调用 ACP 扩展方法（如 star CLI 的 sf/captureSubmit）。
#[derive(Debug, serde::Deserialize)]
struct ExtMethodRequest {
    /// 扩展方法名，不带 `_` 前缀（如 "sf/captureSubmit"）。
    method: String,
    /// 方法参数（JSON 对象），可选的 sessionId 会自动注入。
    #[serde(default)]
    params: serde_json::Value,
}

/// 转发 ACP `extMethod` 请求给当前会话绑定的 CLI 子进程。
///
/// 仅对 ACP 类型 Agent（如 StarCLI）有效；其他类型返回 400。
async fn ext_method(
    State(state): State<ConversationRouterState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
    body: Result<Json<ExtMethodRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let Json(req) = body.map_err(ApiError::from)?;

    // 校验用户拥有该会话
    let conv = state
        .service
        .get(&user.id, &id)
        .await
        .map_err(ApiError::from)?;

    // 获取会话绑定的 Agent 实例
    let instance = state
        .task_manager
        .get_task(&conv.id)
        .ok_or_else(|| ApiError::NotFound("会话的 Agent 未运行，请先发送一条消息初始化会话".into()))?;

    // 仅 ACP Agent 支持 extMethod
    let manager = match instance {
        aionui_ai_agent::AgentInstance::Acp(m) => m,
        _ => {
            return Err(ApiError::BadRequest(
                "extMethod 仅支持 ACP 类型 Agent（如 StarCLI）".into(),
            ));
        }
    };

    // 获取当前 ACP sessionId（可能尚未建立）
    let sid = manager.session_id().await;

    let result = manager
        .ext_request(&req.method, req.params, sid.as_deref())
        .await
        .map_err(|e| match e {
            aionui_ai_agent::AgentError::BadRequest(msg) => ApiError::BadRequest(msg),
            aionui_ai_agent::AgentError::Unauthorized(msg) => ApiError::Unauthorized(msg),
            aionui_ai_agent::AgentError::Forbidden(msg) => ApiError::Forbidden(msg),
            aionui_ai_agent::AgentError::NotFound(msg) => ApiError::NotFound(msg),
            aionui_ai_agent::AgentError::Conflict(msg) => ApiError::Conflict(msg),
            aionui_ai_agent::AgentError::BadGateway(msg) => ApiError::BadGateway(msg),
            aionui_ai_agent::AgentError::Timeout(msg) => ApiError::Timeout(msg),
            aionui_ai_agent::AgentError::RateLimited => ApiError::RateLimited,
            other => ApiError::Internal(other.to_string()),
        })?;

    Ok(Json(ApiResponse::ok(result)))
}
