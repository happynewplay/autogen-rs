use super::base::{Tool, ToolSchema};
use crate::code_executor::func_with_reqs::Import;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type AsyncFn = dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn Error + Send + Sync>>> + Send>> + Send + Sync;

#[derive(Serialize, Deserialize)]
pub struct FunctionToolConfig {
    pub source_code: String,
    pub name: String,
    pub description: String,
    pub global_imports: Vec<Import>,
    pub has_cancellation_support: bool,
}

pub struct FunctionTool {
    func: Arc<AsyncFn>,
    schema: ToolSchema,
    name: String,
    description: String,
}

impl FunctionTool {
    pub fn new(
        func: Arc<AsyncFn>,
        name: String,
        description: String,
        parameters: Value,
    ) -> Self {
        let schema = ToolSchema::new(name.clone(), description.clone(), parameters);
        Self {
            func,
            schema,
            name,
            description,
        }
    }
}

#[async_trait]
impl Tool for FunctionTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn run(&self, args: &Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        (self.func)(args.clone()).await
    }
}