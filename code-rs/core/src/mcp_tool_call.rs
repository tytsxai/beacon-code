use std::time::Instant;

use tracing::error;

use crate::codex::Session;
use crate::codex::ToolCallCtx;
use crate::protocol::EventMsg;
use crate::protocol::McpInvocation;
use crate::protocol::McpToolCallBeginEvent;
use crate::protocol::McpToolCallEndEvent;
use code_protocol::models::ResponseInputItem;

/// Handles the specified tool call dispatches the appropriate
/// `McpToolCallBegin` and `McpToolCallEnd` events to the `Session`.
pub(crate) async fn handle_mcp_tool_call(
    sess: &Session,
    ctx: &ToolCallCtx,
    server: String,
    tool_name: String,
    arguments: String,
) -> ResponseInputItem {
    // Parse the `arguments` as JSON. An empty string is OK, but invalid JSON
    // is not.
    let parsed_arguments = if arguments.trim().is_empty() {
        Ok(None)
    } else {
        serde_json::from_str::<serde_json::Value>(&arguments).map(Some)
    };

    let invocation = McpInvocation {
        server: server.clone(),
        tool: tool_name.clone(),
        arguments: parsed_arguments.as_ref().ok().cloned().flatten(),
    };

    let tool_call_begin_event = EventMsg::McpToolCallBegin(McpToolCallBeginEvent {
        call_id: ctx.call_id.clone(),
        invocation: invocation.clone(),
    });
    notify_mcp_tool_call_event(sess, ctx, tool_call_begin_event).await;

    let start = Instant::now();
    // Perform the tool call.
    let result = match parsed_arguments {
        Ok(arguments_value) => sess
            .call_tool(&server, &tool_name, arguments_value, None)
            .await
            .map_err(|e| format!("tool call error: {e}")),
        Err(err) => {
            error!("failed to parse tool call arguments: {err}");
            Err(format!("tool call error: {err}"))
        }
    };
    let tool_call_end_event = EventMsg::McpToolCallEnd(McpToolCallEndEvent {
        call_id: ctx.call_id.clone(),
        invocation,
        duration: start.elapsed(),
        result: result.clone(),
    });

    notify_mcp_tool_call_event(sess, ctx, tool_call_end_event).await;

    ResponseInputItem::McpToolCallOutput {
        call_id: ctx.call_id.clone(),
        result,
    }
}

async fn notify_mcp_tool_call_event(sess: &Session, ctx: &ToolCallCtx, event: EventMsg) {
    sess.send_ordered_from_ctx(ctx, event).await;
}
