//! Tool wrapper that normalizes parameter schemas for strict OpenAI-compatible APIs.

use std::sync::Arc;

use adk_core::{Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool_schema::ensure_tool_parameters_object;

/// Wraps a tool so `declaration()` / `parameters_schema()` always include `properties`.
pub struct CompatTool {
    inner: Arc<dyn Tool>,
}

impl CompatTool {
    pub fn wrap(inner: Arc<dyn Tool>) -> Arc<dyn Tool> {
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl Tool for CompatTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn declaration(&self) -> Value {
        let mut decl = self.inner.declaration();
        if let Some(params) = decl.get_mut("parameters") {
            *params = ensure_tool_parameters_object(params.clone());
        }
        decl
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
        self.inner
            .parameters_schema()
            .map(ensure_tool_parameters_object)
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
        self.inner.execute(ctx, args).await
    }
}
