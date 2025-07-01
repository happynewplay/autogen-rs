# AutoGen Core Rust 开发指南

本指南详细说明如何将 Python 版本的 autogen-core 转换为 Rust 版本。

## 🎯 项目目标

将功能完整的 Python autogen-core 转换为高性能的 Rust 实现，保持 API 兼容性的同时提供：
- 更高的性能和更低的内存使用
- 类型安全和内存安全
- 更好的并发处理能力
- 与现有 Python 生态系统的互操作性

## 📁 项目结构

```
rust/
├── Cargo.toml                 # 工作空间配置
├── autogen-core/              # 核心库
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs            # 库入口
│   │   ├── agent.rs          # Agent trait 和相关类型
│   │   ├── agent_id.rs       # Agent 标识符
│   │   ├── agent_runtime.rs  # Runtime trait
│   │   ├── message.rs        # 消息系统
│   │   ├── subscription.rs   # 订阅系统
│   │   ├── topic.rs          # 主题系统
│   │   ├── base_agent.rs     # 基础 Agent 实现
│   │   ├── closure_agent.rs  # 闭包 Agent
│   │   ├── routed_agent.rs   # 路由 Agent
│   │   ├── single_threaded_runtime.rs # 单线程运行时
│   │   ├── serialization.rs  # 序列化
│   │   ├── tools/            # 工具系统
│   │   ├── models/           # 模型集成
│   │   ├── memory/           # 内存管理
│   │   ├── cache.rs          # 缓存系统
│   │   ├── cancellation.rs   # 取消令牌
│   │   ├── component.rs      # 组件系统
│   │   ├── telemetry.rs      # 遥测
│   │   └── error.rs          # 错误类型
│   ├── examples/             # 示例代码
│   └── benches/              # 性能测试
├── autogen-agentchat/         # AgentChat 实现
├── autogen-ext/               # 扩展库
└── examples/                  # 完整示例
```

## 🔧 开发环境设置

### 1. 安装 Rust 工具链

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装必要组件
rustup component add clippy rustfmt
rustup component add llvm-tools-preview

# 安装开发工具
cargo install cargo-watch cargo-expand cargo-audit
```

### 2. 克隆并设置项目

```bash
cd rust/
cargo check  # 检查依赖
cargo test   # 运行测试
cargo fmt    # 格式化代码
cargo clippy # 代码检查
```

## 📋 开发任务清单

### 阶段 1: 核心基础设施 (已完成)
- [x] 项目结构和工作空间设置
- [x] 核心依赖配置
- [x] 基础类型定义 (AgentId, Error)
- [x] Agent trait 定义

### 阶段 2: 消息系统 (进行中)
- [ ] MessageContext 实现
- [ ] Topic 和 TopicId 实现
- [ ] Subscription 系统
- [ ] 消息路由机制
- [ ] 序列化框架

### 阶段 3: Runtime 系统
- [ ] AgentRuntime trait
- [ ] SingleThreadedAgentRuntime
- [ ] Agent 生命周期管理
- [ ] 消息分发机制

### 阶段 4: Agent 实现
- [ ] BaseAgent
- [ ] ClosureAgent
- [ ] RoutedAgent
- [ ] ToolAgent

### 阶段 5: 支持系统
- [ ] 工具调用框架
- [ ] 模型客户端接口
- [ ] 内存管理
- [ ] 缓存系统
- [ ] 遥测集成

## 🔄 Python 到 Rust 转换模式

### 1. 类型转换

| Python | Rust |
|--------|------|
| `str` | `String` / `&str` |
| `int` | `i32` / `i64` / `usize` |
| `float` | `f32` / `f64` |
| `bool` | `bool` |
| `List[T]` | `Vec<T>` |
| `Dict[K, V]` | `HashMap<K, V>` |
| `Optional[T]` | `Option<T>` |
| `Union[T, U]` | `enum` |

### 2. 异步模式

```python
# Python
async def handle_message(self, message):
    await self.process(message)
```

```rust
// Rust
async fn handle_message(&mut self, message: Message) -> Result<()> {
    self.process(message).await
}
```

### 3. 错误处理

```python
# Python
try:
    result = await operation()
except Exception as e:
    handle_error(e)
```

```rust
// Rust
match operation().await {
    Ok(result) => handle_success(result),
    Err(e) => handle_error(e),
}
```

## 🧪 测试策略

### 1. 单元测试
每个模块都应该有对应的测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        // 测试代码
    }
}
```

### 2. 集成测试
在 `tests/` 目录下创建集成测试：

```rust
// tests/integration_test.rs
use autogen_core::*;

#[tokio::test]
async fn test_full_agent_workflow() {
    // 端到端测试
}
```

### 3. 性能测试
使用 Criterion 进行性能测试：

```rust
// benches/agent_runtime.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_message_passing(c: &mut Criterion) {
    c.bench_function("message_passing", |b| {
        b.iter(|| {
            // 性能测试代码
        })
    });
}

criterion_group!(benches, benchmark_message_passing);
criterion_main!(benches);
```

## 📚 参考资源

### Rust 学习资源
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Async Programming in Rust](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### 相关 Crates
- `tokio` - 异步运行时
- `serde` - 序列化
- `tracing` - 日志和追踪
- `async-trait` - 异步 trait
- `thiserror` - 错误处理
- `prost` - Protocol Buffers

### Python 参考
- [autogen-core Python 源码](../python/packages/autogen-core/src/autogen_core/)
- [Python API 文档](https://microsoft.github.io/autogen/stable/user-guide/core-user-guide/)

## 🚀 下一步行动

1. **立即开始**: 运行 `cargo test` 确保基础设施正常工作
2. **选择任务**: 从任务清单中选择一个未完成的任务
3. **参考 Python**: 查看对应的 Python 实现
4. **实现功能**: 使用 Rust 实现相同功能
5. **编写测试**: 确保功能正确性
6. **性能优化**: 利用 Rust 的性能优势

## 💡 开发技巧

1. **渐进式开发**: 先实现基本功能，再添加高级特性
2. **类型驱动**: 先定义类型和 trait，再实现具体逻辑
3. **测试先行**: 为每个功能编写测试
4. **文档同步**: 保持代码文档的更新
5. **性能意识**: 利用 Rust 的零成本抽象

开始你的 Rust 转换之旅吧！🦀
