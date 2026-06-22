//! Rate-limit helpers: mandatory cooldown after each LLM turn and tool execution.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use adk_core::{Llm, LlmRequest, LlmResponse, LlmResponseStream, Result, Tool, ToolContext};

use crate::compat_tool::CompatTool;
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use tokio::time::Sleep;

/// Default pause after each LLM turn / tool call (reduces 429 rate limits).
pub const DEFAULT_CALL_COOLDOWN: Duration = Duration::from_millis(200);

pub fn call_cooldown_from_env() -> Duration {
    std::env::var("TERRAIN_LLM_COOLDOWN_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_CALL_COOLDOWN)
}

pub fn wrap_llm(inner: Arc<dyn Llm>, cooldown: Duration) -> Arc<dyn Llm> {
    Arc::new(ThrottledLlm { inner, cooldown })
}

pub fn wrap_tool(inner: Arc<dyn Tool>, cooldown: Duration) -> Arc<dyn Tool> {
    let inner = CompatTool::wrap(inner);
    Arc::new(ThrottledTool { inner, cooldown })
}

fn execution_times() -> &'static Mutex<HashMap<String, u64>> {
    static STORE: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Wall-clock tool execution time (excludes post-call throttle sleep).
pub fn take_tool_execution_ms(call_id: &str) -> Option<u64> {
    execution_times().lock().ok()?.remove(call_id)
}

struct ThrottledLlm {
    inner: Arc<dyn Llm>,
    cooldown: Duration,
}

struct ThrottledLlmStream {
    inner: LlmResponseStream,
    cooldown: Duration,
    sleep: Option<Pin<Box<Sleep>>>,
    delayed_this_turn: bool,
}

impl Stream for ThrottledLlmStream {
    type Item = Result<LlmResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(sleep) = self.sleep.as_mut() {
            if sleep.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.sleep = None;
            self.delayed_this_turn = true;
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if item
                    .as_ref()
                    .ok()
                    .is_some_and(|response| response.turn_complete)
                    && !self.delayed_this_turn
                {
                    self.sleep = Some(Box::pin(tokio::time::sleep(self.cooldown)));
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) if !self.delayed_this_turn => {
                self.sleep = Some(Box::pin(tokio::time::sleep(self.cooldown)));
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            other => other,
        }
    }
}

#[async_trait]
impl Llm for ThrottledLlm {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate_content(&self, req: LlmRequest, stream: bool) -> Result<LlmResponseStream> {
        let inner = self.inner.generate_content(req, stream).await?;
        Ok(Box::pin(ThrottledLlmStream {
            inner,
            cooldown: self.cooldown,
            sleep: None,
            delayed_this_turn: false,
        }))
    }

    fn schema_adapter(&self) -> &dyn adk_core::SchemaAdapter {
        self.inner.schema_adapter()
    }

    fn uses_interactions_api(&self) -> bool {
        self.inner.uses_interactions_api()
    }
}

struct ThrottledTool {
    inner: Arc<dyn Tool>,
    cooldown: Duration,
}

#[async_trait]
impl Tool for ThrottledTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn declaration(&self) -> Value {
        self.inner.declaration()
    }

    fn enhanced_description(&self) -> String {
        self.inner.enhanced_description()
    }

    fn is_long_running(&self) -> bool {
        self.inner.is_long_running()
    }

    fn is_builtin(&self) -> bool {
        self.inner.is_builtin()
    }

    fn parameters_schema(&self) -> Option<Value> {
        self.inner.parameters_schema()
    }

    fn response_schema(&self) -> Option<Value> {
        self.inner.response_schema()
    }

    fn required_scopes(&self) -> &[&str] {
        self.inner.required_scopes()
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    fn is_concurrency_safe(&self) -> bool {
        self.inner.is_concurrency_safe()
    }

    async fn execute(&self, ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let call_id = ctx.function_call_id().to_string();
        let started = Instant::now();
        let result = self.inner.execute(ctx, args).await;
        if let Ok(mut map) = execution_times().lock() {
            map.insert(call_id, started.elapsed().as_millis() as u64);
        }
        tokio::time::sleep(self.cooldown).await;
        result
    }
}
