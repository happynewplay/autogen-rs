# Rust AutoGen-Core 快速开始指南

## 🚀 立即开始第一个任务

基于分析报告，我们从修复运行时的"未实现"功能开始。这是最关键的第一步。

## 📋 准备工作

### 1. 环境检查
```bash
# 检查 Rust 版本
rustc --version  # 建议 1.70+

# 检查项目编译
cd rust/autogen-core
cargo check

# 安装开发工具
cargo install cargo-watch cargo-nextest
```

### 2. 代码质量工具
```bash
# 格式化代码
cargo fmt

# 检查代码质量
cargo clippy -- -D warnings

# 运行测试
cargo test
```

## 🎯 第一个任务：修复 Intervention Handler

### 当前问题
在 `single_threaded_agent_runtime.rs` 中有这些"未实现"标记：
```rust
// 未实现: 简化intervention handler处理，暂时跳过
// let message_context = MessageContext { ... };
// for handler in &intervention_handlers { ... }
```

### 实施步骤

#### 步骤 1: 设计 Intervention 系统
创建或修改 `rust/autogen-core/src/intervention.rs`:

```rust
use crate::agent_id::AgentId;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct InterventionContext {
    pub message: Value,
    pub sender: Option<AgentId>,
    pub recipient: AgentId,
    pub metadata: HashMap<String, Value>,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub enum InterventionAction {
    /// 继续处理消息
    Continue,
    /// 修改消息内容
    ModifyMessage(Value),
    /// 丢弃消息
    DropMessage,
    /// 重定向到其他 Agent
    Redirect(AgentId),
    /// 延迟处理
    Delay(std::time::Duration),
}

#[async_trait]
pub trait InterventionHandler: Send + Sync {
    /// 处理消息拦截
    async fn handle(&self, ctx: &mut InterventionContext) -> Result<InterventionAction, Box<dyn Error>>;
    
    /// 获取处理器名称
    fn name(&self) -> &str;
    
    /// 获取处理器优先级（数字越小优先级越高）
    fn priority(&self) -> i32 {
        0
    }
}

/// 默认的拦截处理器，什么都不做
#[derive(Debug, Default)]
pub struct DefaultInterventionHandler;

#[async_trait]
impl InterventionHandler for DefaultInterventionHandler {
    async fn handle(&self, _ctx: &mut InterventionContext) -> Result<InterventionAction, Box<dyn Error>> {
        Ok(InterventionAction::Continue)
    }
    
    fn name(&self) -> &str {
        "default"
    }
}

/// 丢弃消息的处理器
#[derive(Debug)]
pub struct DropMessage;

#[async_trait]
impl InterventionHandler for DropMessage {
    async fn handle(&self, _ctx: &mut InterventionContext) -> Result<InterventionAction, Box<dyn Error>> {
        Ok(InterventionAction::DropMessage)
    }
    
    fn name(&self) -> &str {
        "drop_message"
    }
}
```

#### 步骤 2: 修改运行时实现
在 `single_threaded_agent_runtime.rs` 中：

1. **添加字段**:
```rust
pub struct SingleThreadedAgentRuntime {
    // ... 现有字段
    intervention_handlers: Vec<Box<dyn InterventionHandler>>,
}
```

2. **修改构造函数**:
```rust
impl SingleThreadedAgentRuntime {
    pub fn new() -> Self {
        Self {
            // ... 现有字段初始化
            intervention_handlers: vec![Box::new(DefaultInterventionHandler)],
        }
    }
    
    pub fn add_intervention_handler(&mut self, handler: Box<dyn InterventionHandler>) {
        self.intervention_handlers.push(handler);
        // 按优先级排序
        self.intervention_handlers.sort_by_key(|h| h.priority());
    }
}
```

3. **实现拦截逻辑**:
```rust
async fn apply_intervention_handlers(
    handlers: &[Box<dyn InterventionHandler>],
    mut context: InterventionContext,
) -> Result<(Value, Option<AgentId>), Box<dyn Error>> {
    for handler in handlers {
        match handler.handle(&mut context).await? {
            InterventionAction::Continue => continue,
            InterventionAction::ModifyMessage(new_message) => {
                context.message = new_message;
            }
            InterventionAction::DropMessage => {
                return Err("Message dropped by intervention handler".into());
            }
            InterventionAction::Redirect(new_recipient) => {
                context.recipient = new_recipient;
            }
            InterventionAction::Delay(duration) => {
                tokio::time::sleep(duration).await;
            }
        }
    }
    Ok((context.message, Some(context.recipient)))
}
```

4. **替换"未实现"部分**:
```rust
// 替换这部分代码：
// 未实现: 简化intervention handler处理，暂时跳过

// 改为：
let intervention_context = InterventionContext {
    message: envelope.message.clone(),
    sender: envelope.sender.clone(),
    recipient: envelope.recipient.clone(),
    metadata: HashMap::new(),
    timestamp: std::time::SystemTime::now(),
};

let (processed_message, final_recipient) = apply_intervention_handlers(
    &self.intervention_handlers,
    intervention_context,
).await?;
```

#### 步骤 3: 编写测试
创建 `rust/autogen-core/src/intervention.rs` 的测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_default_intervention_handler() {
        let handler = DefaultInterventionHandler;
        let mut ctx = InterventionContext {
            message: serde_json::json!({"test": "message"}),
            sender: None,
            recipient: AgentId::new("test", None),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        
        let result = handler.handle(&mut ctx).await.unwrap();
        matches!(result, InterventionAction::Continue);
    }

    #[tokio::test]
    async fn test_drop_message_handler() {
        let handler = DropMessage;
        let mut ctx = InterventionContext {
            message: serde_json::json!({"test": "message"}),
            sender: None,
            recipient: AgentId::new("test", None),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        
        let result = handler.handle(&mut ctx).await.unwrap();
        matches!(result, InterventionAction::DropMessage);
    }
}
```

## ✅ 验证步骤

1. **编译检查**:
```bash
cargo check
```

2. **运行测试**:
```bash
cargo test intervention
```

3. **代码质量检查**:
```bash
cargo clippy
cargo fmt
```

4. **集成测试**:
```bash
cargo test --test basic_compilation_test
```

## 📝 完成标准

- [ ] 所有"未实现: 简化intervention handler处理"标记被移除
- [ ] 新的 intervention 系统编译通过
- [ ] 所有测试通过
- [ ] 代码通过 clippy 检查
- [ ] 添加了适当的文档注释

## 🔄 下一步

完成这个任务后，继续进行：
1. 订阅管理系统的实现
2. 状态持久化功能
3. 异步绑定问题的解决

## 💡 提示

- 使用 `cargo watch -x check` 进行实时编译检查
- 遇到 Send/Sync 问题时，考虑使用 `Arc<dyn Trait + Send + Sync>`
- 保持小步快跑，每个功能都要有对应的测试

---

**开始时间**: 立即
**预估完成时间**: 1-2 天
**优先级**: 最高
