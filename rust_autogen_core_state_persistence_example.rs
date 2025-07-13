// Rust AutoGen-Core 状态持久化功能使用示例
// 
// 这个示例展示了如何使用新实现的状态持久化功能

use autogen_core::agent::{Agent, AgentFactory};
use autogen_core::agent_id::AgentId;
use autogen_core::agent_type::AgentType;
use autogen_core::single_threaded_agent_runtime::SingleThreadedAgentRuntime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建运行时
    let mut runtime = SingleThreadedAgentRuntime::new(vec![]);
    
    // 2. 注册 agent factory（如果需要）
    // let agent_type = AgentType { r#type: "example_agent".to_string() };
    // let factory = ExampleAgentFactory::new();
    // runtime.register_factory(agent_type, Box::new(factory)).await?;
    
    // 3. 保存当前状态
    println!("保存运行时状态...");
    let saved_state = runtime.save_state().await?;
    
    // 4. 显示保存的状态信息
    println!("状态保存成功！");
    println!("- 运行时类型: {:?}", saved_state.get("runtime_type"));
    println!("- 架构版本: {:?}", saved_state.get("schema_version"));
    println!("- 版本: {:?}", saved_state.get("version"));
    
    if let Some(integrity) = saved_state.get("state_integrity") {
        if let Some(integrity_obj) = integrity.as_object() {
            println!("- Agents 数量: {:?}", integrity_obj.get("agents_count"));
            println!("- 工厂数量: {:?}", integrity_obj.get("factories_count"));
            println!("- 校验和: {:?}", integrity_obj.get("checksum"));
        }
    }
    
    // 5. 加载状态
    println!("\n加载运行时状态...");
    runtime.load_state(&saved_state).await?;
    println!("状态加载成功！");
    
    // 6. 演示版本迁移功能
    println!("\n演示版本迁移功能...");
    
    // 创建一个模拟的 v1.0 状态
    let mut v1_state = HashMap::new();
    v1_state.insert("runtime_type".to_string(), Value::String("SingleThreadedAgentRuntime".to_string()));
    v1_state.insert("schema_version".to_string(), Value::String("1.0".to_string()));
    v1_state.insert("version".to_string(), Value::String("1.0.0".to_string()));
    v1_state.insert("saved_at".to_string(), Value::Number(1234567890.into()));
    
    // 添加 v1.0 格式的 agents
    let mut agents = serde_json::Map::new();
    agents.insert("example_agent/default".to_string(), serde_json::json!({
        "id": "example_agent/default",
        "type": "example_agent",
        "created_at": 1234567890
    }));
    v1_state.insert("agents".to_string(), Value::Object(agents));
    
    // 添加 v1.0 格式的订阅（数组格式）
    v1_state.insert("subscriptions".to_string(), serde_json::json!(["sub1", "sub2"]));
    
    // 尝试加载 v1.0 状态（会自动迁移到 v2.0）
    match runtime.load_state(&v1_state).await {
        Ok(_) => println!("v1.0 状态成功迁移并加载！"),
        Err(e) => println!("迁移失败: {}", e),
    }
    
    // 7. 演示状态完整性验证
    println!("\n演示状态完整性验证...");
    
    let mut tampered_state = saved_state.clone();
    tampered_state.insert("tampered_field".to_string(), Value::String("tampered_value".to_string()));
    
    match runtime.load_state(&tampered_state).await {
        Ok(_) => println!("意外：被篡改的状态竟然加载成功了"),
        Err(e) => println!("正确：检测到状态被篡改，拒绝加载: {}", e),
    }
    
    // 8. 演示不支持版本的处理
    println!("\n演示不支持版本的处理...");
    
    let mut unsupported_state = HashMap::new();
    unsupported_state.insert("runtime_type".to_string(), Value::String("SingleThreadedAgentRuntime".to_string()));
    unsupported_state.insert("schema_version".to_string(), Value::String("99.0".to_string()));
    unsupported_state.insert("version".to_string(), Value::String("99.0.0".to_string()));
    unsupported_state.insert("saved_at".to_string(), Value::Number(1234567890.into()));
    
    match runtime.load_state(&unsupported_state).await {
        Ok(_) => println!("意外：不支持的版本竟然加载成功了"),
        Err(e) => println!("正确：检测到不支持的版本，拒绝加载: {}", e),
    }
    
    println!("\n状态持久化功能演示完成！");
    
    Ok(())
}

// 示例 Agent 实现（用于演示）
#[derive(Clone)]
struct ExampleAgent {
    id: AgentId,
    data: String,
}

impl ExampleAgent {
    fn new(id: AgentId) -> Self {
        Self {
            id,
            data: "example_data".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Agent for ExampleAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }

    fn metadata(&self) -> autogen_core::agent_metadata::AgentMetadata {
        autogen_core::agent_metadata::AgentMetadata {
            r#type: self.id.r#type.clone(),
            key: self.id.key.clone(),
            description: "Example agent for state persistence demo".to_string(),
        }
    }

    fn id(&self) -> AgentId {
        self.id.clone()
    }

    async fn bind_id_and_runtime(
        &mut self,
        _id: AgentId,
        _runtime: &dyn autogen_core::agent_runtime::AgentRuntime,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn on_message(
        &mut self,
        message: Value,
        _ctx: autogen_core::message_context::MessageContext,
    ) -> Result<Value, Box<dyn std::error::Error + Send>> {
        Ok(message)
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
        let mut state = HashMap::new();
        state.insert("agent_id".to_string(), serde_json::to_value(&self.id)?);
        state.insert("data".to_string(), Value::String(self.data.clone()));
        Ok(state)
    }

    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(data_value) = state.get("data") {
            if let Some(data) = data_value.as_str() {
                self.data = data.to_string();
            }
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// 示例 Agent Factory 实现
#[derive(Clone)]
struct ExampleAgentFactory;

impl ExampleAgentFactory {
    fn new() -> Self {
        Self
    }
}

impl AgentFactory for ExampleAgentFactory {
    fn clone_box(&self) -> Box<dyn AgentFactory> {
        Box::new(self.clone())
    }

    fn create(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Agent>, Box<dyn std::error::Error>>> + Send>> {
        Box::pin(async move {
            let agent_id = AgentId {
                r#type: "example_agent".to_string(),
                key: "default".to_string(),
            };
            Ok(Box::new(ExampleAgent::new(agent_id)) as Box<dyn Agent>)
        })
    }
}
