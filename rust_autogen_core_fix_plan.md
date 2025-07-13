# Rust AutoGen-Core 代码修复计划

## 📋 计划概述

基于功能差异分析报告，制定分阶段的代码修复计划，优先解决严重缺失的核心功能，逐步完善整个系统。

## 🎯 第一阶段：核心运行时修复 (高优先级)

### 任务 1.1: 修复运行时"未实现"功能
**文件**: `rust/autogen-core/src/single_threaded_agent_runtime.rs`
**预估工作量**: 3-5 天

**具体任务**:
1. **实现 Intervention Handler 处理**
   - 添加消息拦截逻辑
   - 实现处理链模式
   - 支持消息修改和丢弃

2. **完善订阅管理系统**
   - 实现 `add_subscription` 的实际逻辑
   - 实现 `remove_subscription` 的实际逻辑
   - 添加订阅存储和查询机制

3. **实现状态持久化**
   - 完善 `save_state` 方法
   - 添加 `load_state` 方法
   - 实现 Agent 状态序列化

4. **解决异步绑定问题**
   - 重构 Agent 绑定逻辑
   - 解决 Send trait 约束问题
   - 确保线程安全

### 任务 1.2: 完善消息处理机制
**文件**: `rust/autogen-core/src/single_threaded_agent_runtime.rs`, `rust/autogen-core/src/queue.rs`
**预估工作量**: 2-3 天

**具体任务**:
1. **消息路由优化**
   - 实现智能消息分发
   - 添加路由规则引擎
   - 支持消息优先级

2. **错误处理增强**
   - 添加重试机制
   - 实现错误恢复策略
   - 完善异常传播

## 🎯 第二阶段：模型客户端系统 (高优先级)

### 任务 2.1: 扩展模型类型系统
**文件**: `rust/autogen-core/src/models/types.rs`
**预估工作量**: 2-3 天

**具体任务**:
1. **添加缺失的响应类型**
   ```rust
   // 需要添加的类型
   pub struct RequestUsage { /* ... */ }
   pub enum FinishReasons { /* ... */ }
   pub struct CreateResult { /* ... */ }
   pub struct TopLogprob { /* ... */ }
   pub struct ChatCompletionTokenLogprob { /* ... */ }
   ```

2. **增强消息类型**
   - 支持更复杂的内容类型
   - 添加元数据支持
   - 实现消息验证

### 任务 2.2: 实现模型能力系统
**文件**: `rust/autogen-core/src/models/model_client.rs` (新建相关文件)
**预估工作量**: 3-4 天

**具体任务**:
1. **创建模型信息系统**
   ```rust
   // 需要实现的结构
   pub struct ModelCapabilities { /* ... */ }
   pub enum ModelFamily { /* ... */ }
   pub struct ModelInfo { /* ... */ }
   pub fn validate_model_info(info: &ModelInfo) -> Result<(), Error>;
   ```

2. **扩展 ChatCompletionClient**
   - 添加 `create` 方法
   - 支持流式处理
   - 实现工具调用
   - 添加结构化输出支持

## 🎯 第三阶段：订阅和消息系统 (中优先级)

### 任务 3.1: 完善订阅管理器
**文件**: `rust/autogen-core/src/subscription_manager.rs`
**预估工作量**: 2-3 天

**具体任务**:
1. **实现订阅存储**
   - 添加持久化订阅存储
   - 实现订阅查询优化
   - 支持动态订阅更新

2. **优化匹配算法**
   - 实现高效的主题匹配
   - 支持通配符和正则表达式
   - 添加订阅优先级

### 任务 3.2: 增强消息上下文
**文件**: `rust/autogen-core/src/message_context.rs`, `rust/autogen-core/src/message_handler_context.rs`
**预估工作量**: 1-2 天

**具体任务**:
1. **扩展上下文信息**
   - 添加更多元数据
   - 支持上下文传播
   - 实现上下文验证

## 🎯 第四阶段：组件配置系统 (中优先级)

### 任务 4.1: 实现组件配置管理
**文件**: `rust/autogen-core/src/component_config.rs`
**预估工作量**: 3-4 天

**具体任务**:
1. **创建配置加载器**
   ```rust
   // 需要实现的功能
   pub struct ComponentLoader { /* ... */ }
   pub trait ComponentFromConfig { /* ... */ }
   pub trait ComponentToConfig { /* ... */ }
   ```

2. **添加组件工具函数**
   ```rust
   pub fn is_component_class<T>() -> bool;
   pub fn is_component_instance<T>(obj: &T) -> bool;
   ```

3. **实现生命周期管理**
   - 组件初始化
   - 配置验证
   - 依赖注入

## 🎯 第五阶段：内存和代码执行 (中优先级)

### 任务 5.1: 实现内存管理系统
**文件**: `rust/autogen-core/src/memory/list_memory.rs` (新建实现)
**预估工作量**: 2-3 天

**具体任务**:
1. **实现 ListMemory**
   - 基础内存存储
   - 查询和索引
   - 与模型上下文集成

### 任务 5.2: 完善代码执行器
**文件**: `rust/autogen-core/src/code_executor/`
**预估工作量**: 4-5 天

**具体任务**:
1. **实现安全执行环境**
   - 沙箱隔离
   - 资源限制
   - 依赖管理

## 🎯 第六阶段：遥测和工具系统 (低优先级)

### 任务 6.1: 完善遥测集成
**文件**: `rust/autogen-core/src/telemetry/`
**预估工作量**: 2-3 天

### 任务 6.2: 增强工具系统
**文件**: `rust/autogen-core/src/tools/`
**预估工作量**: 2-3 天

## 📅 时间计划

| 阶段 | 预估时间 | 关键里程碑 |
|------|----------|------------|
| 第一阶段 | 1-2 周 | 运行时核心功能可用 |
| 第二阶段 | 1-1.5 周 | 模型客户端基本可用 |
| 第三阶段 | 1 周 | 消息系统完善 |
| 第四阶段 | 1 周 | 组件配置可用 |
| 第五阶段 | 1.5 周 | 内存和执行器可用 |
| 第六阶段 | 1 周 | 系统功能完整 |

**总预估时间**: 6-8 周

## 🧪 测试策略

1. **单元测试**: 每个修复的模块都要有对应的单元测试
2. **集成测试**: 关键功能的端到端测试
3. **性能测试**: 与 Python 版本的性能对比
4. **兼容性测试**: 确保 API 兼容性

## 📋 验收标准

1. **功能完整性**: 所有"未实现"标记被移除
2. **测试覆盖率**: 核心功能测试覆盖率 > 80%
3. **文档完整性**: 所有公共 API 都有文档
4. **性能基准**: 关键操作性能不低于 Python 版本

## 🛠️ 实施指导

### 开发环境准备
1. **依赖检查**: 确保 `Cargo.toml` 中的依赖版本兼容
2. **开发工具**: 安装 `cargo-watch`, `cargo-nextest` 等开发工具
3. **IDE 配置**: 配置 rust-analyzer 和相关插件

### 代码质量标准
1. **代码风格**: 使用 `rustfmt` 和 `clippy`
2. **错误处理**: 统一使用 `thiserror` 和 `anyhow`
3. **异步编程**: 遵循 Tokio 最佳实践
4. **文档**: 所有公共 API 必须有 rustdoc 注释

### 第一阶段详细实施步骤

#### 步骤 1: 修复 Intervention Handler
```rust
// 在 single_threaded_agent_runtime.rs 中添加
pub struct InterventionContext {
    pub message: Value,
    pub sender: Option<AgentId>,
    pub recipient: AgentId,
    pub metadata: HashMap<String, Value>,
}

pub trait InterventionHandler: Send + Sync {
    async fn handle(&self, ctx: &mut InterventionContext) -> Result<InterventionAction, Box<dyn Error>>;
}

pub enum InterventionAction {
    Continue,
    ModifyMessage(Value),
    DropMessage,
    Redirect(AgentId),
}
```

#### 步骤 2: 实现订阅管理
```rust
// 在 subscription_manager.rs 中实现
impl SubscriptionManager {
    pub fn add_subscription(&mut self, subscription: Box<dyn Subscription>) -> Result<(), Error> {
        // 实际的订阅添加逻辑
        let id = subscription.id();
        self.subscriptions.insert(id, subscription);
        self.rebuild_index();
        Ok(())
    }

    pub fn remove_subscription(&mut self, id: &str) -> Result<(), Error> {
        // 实际的订阅移除逻辑
        self.subscriptions.remove(id);
        self.rebuild_index();
        Ok(())
    }
}
```

### 风险评估与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Send/Sync 约束问题 | 高 | 使用 Arc<Mutex<>> 或 Arc<RwLock<>> |
| 异步生命周期复杂 | 中 | 分步重构，先同步后异步 |
| 内存泄漏 | 中 | 使用 Weak 引用打破循环 |
| 性能回归 | 低 | 持续性能测试 |

## 📊 进度跟踪

### 里程碑检查点
- [ ] **M1**: 运行时"未实现"功能全部修复
- [ ] **M2**: 基础模型客户端可用
- [ ] **M3**: 消息系统完全功能
- [ ] **M4**: 组件配置系统可用
- [ ] **M5**: 内存和执行器基本可用
- [ ] **M6**: 系统功能完整，通过所有测试

### 每日检查清单
- [ ] 代码编译无警告
- [ ] 新增测试通过
- [ ] 文档更新完成
- [ ] 性能基准测试通过

## 🚀 开始建议

**立即开始**: 从**第一阶段任务 1.1**开始，优先修复运行时的"未实现"功能
**并行开发**: 可以同时进行模型类型系统的设计工作
**持续集成**: 每完成一个任务就进行集成测试

---

**计划制定日期**: 2025-07-12
**计划版本**: v1.0
**下次评估**: 第一阶段完成后
