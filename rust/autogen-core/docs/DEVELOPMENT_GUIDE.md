# AutoGen Core Rust - Development Guide

This guide covers the development practices, architecture decisions, and contribution guidelines for the AutoGen Core Rust implementation.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Type Safety Improvements](#type-safety-improvements)
- [Feature Gates](#feature-gates)
- [Error Handling](#error-handling)
- [Testing Strategy](#testing-strategy)
- [Performance Considerations](#performance-considerations)
- [Contributing](#contributing)

## Architecture Overview

### Core Components

The AutoGen Core Rust library is organized into several key modules:

1. **Message System** (`message.rs`)
   - Type-safe message handling with `Message` trait
   - Typed and untyped message envelopes
   - Compile-time type checking for message routing

2. **Agent System** (`agent.rs`)
   - `TypedAgent<M>` trait for type-safe message handling
   - Legacy `Agent` trait for backward compatibility
   - Agent adapters for seamless integration

3. **Memory Management** (`memory/`)
   - Content validation and size limits
   - Multiple content types (text, JSON, binary, images)
   - Pluggable memory backends

4. **Tool System** (`tools/`)
   - Type-safe tool definitions with `TypedTool<Args, Return>`
   - Argument validation and schema generation
   - Tool adapters for legacy compatibility

5. **Error Handling** (`error.rs`)
   - Structured error types with context
   - Error chain preservation
   - Recovery strategies

## Type Safety Improvements

### Before (Legacy)

```rust
// Type-erased message handling
async fn handle_message(
    &mut self,
    message: Box<dyn Any + Send>,
    ctx: &MessageContext,
) -> Result<Option<Box<dyn Any + Send>>>;
```

### After (Type-Safe)

```rust
// Type-safe message handling
async fn handle_message(
    &mut self,
    message: M,
    ctx: &MessageContext,
) -> Result<Option<M::Response>>;
```

### Benefits

- **Compile-time guarantees**: Message type mismatches are caught at compile time
- **Better IDE support**: Full autocomplete and type checking
- **Reduced runtime errors**: No more failed downcasts
- **Performance**: Eliminates boxing/unboxing overhead

## Feature Gates

The library uses feature gates to allow minimal builds:

### Core Features

- `default = ["runtime", "json", "validation"]`
- `minimal = ["json"]` - Minimal feature set
- `full = [...]` - All features enabled

### Optional Features

- `runtime` - Agent runtime system (requires tokio)
- `json` - JSON serialization support
- `protobuf` - Protocol Buffer support
- `http` - HTTP client functionality
- `telemetry` - Tracing and observability
- `images` - Image processing support
- `config-support` - Configuration management

### Usage

```toml
[dependencies]
autogen-core = { version = "0.6.2", features = ["minimal"] }
```

## Error Handling

### Error Context

All errors include rich context information:

```rust
let context = ErrorContext::new("agent_message_handling")
    .with_detail("agent_id", agent.id().to_string())
    .with_detail("message_type", "TextMessage");

let error = AutoGenError::agent(context, source_error);
```

### Error Types

- **Structured errors**: Specific error types for different failure modes
- **Error chains**: Preserve underlying error causes
- **Recovery hints**: Include suggestions for error resolution

### Best Practices

1. Always include context when creating errors
2. Use specific error types rather than generic ones
3. Preserve error chains for debugging
4. Include recovery suggestions where possible

## Testing Strategy

### Test Categories

1. **Unit Tests** - Individual component testing
2. **Integration Tests** - Component interaction testing
3. **Error Path Tests** - Failure scenario testing
4. **Performance Tests** - Benchmark critical paths

### Test Organization

```
tests/
├── integration_tests.rs    # Cross-component integration
├── error_tests.rs         # Error handling scenarios
├── test_utils.rs          # Shared test utilities
└── ...

benches/
└── performance_benchmarks.rs  # Performance benchmarks
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test category
cargo test integration_tests

# Run benchmarks
cargo bench

# Test with different feature combinations
cargo test --no-default-features --features minimal
cargo test --features full
```

## Performance Considerations

### Design Decisions

1. **Reduced Allocations**: Minimize boxing and heap allocations
2. **Zero-Copy**: Use references where possible
3. **Compile-Time Optimization**: Leverage Rust's type system
4. **Async Efficiency**: Proper async/await usage

### Benchmarking

Key metrics tracked:

- Message creation and routing latency
- Memory allocation patterns
- Concurrent message handling throughput
- Agent startup time

### Optimization Guidelines

1. Profile before optimizing
2. Measure the impact of changes
3. Consider memory vs. CPU trade-offs
4. Use `#[inline]` judiciously

## Contributing

### Development Setup

1. **Install Rust**: Use rustup for the latest stable version
2. **Clone Repository**: `git clone <repo-url>`
3. **Install Dependencies**: `cargo build`
4. **Run Tests**: `cargo test`

### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Use clippy for linting (`cargo clippy`)
- Write comprehensive documentation
- Include examples in doc comments

### Pull Request Process

1. **Create Feature Branch**: `git checkout -b feature/your-feature`
2. **Write Tests**: Ensure good test coverage
3. **Update Documentation**: Keep docs current
4. **Run CI Checks**: `cargo test && cargo clippy && cargo fmt --check`
5. **Submit PR**: Include clear description and rationale

### Documentation Standards

- Use `///` for public API documentation
- Include examples in doc comments
- Document error conditions
- Explain design decisions in module-level docs

### Commit Message Format

```
type(scope): brief description

Longer explanation if needed.

- List specific changes
- Reference issues: Fixes #123
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## Migration from Python

### Key Differences

1. **Type Safety**: Rust version provides compile-time guarantees
2. **Memory Management**: Automatic memory safety without GC
3. **Performance**: Significantly faster execution
4. **Error Handling**: Structured error types vs. exceptions

### Migration Strategy

1. **Start with Core Types**: AgentId, TopicId, basic messages
2. **Implement Agents**: Use TypedAgent for new agents
3. **Add Tools**: Leverage type-safe tool system
4. **Integrate Memory**: Use validated memory content
5. **Handle Errors**: Adopt structured error handling

### Compatibility

- API compatibility maintained where possible
- Legacy traits available for gradual migration
- Adapter patterns for seamless integration

## Best Practices

### Agent Design

1. **Single Responsibility**: Each agent should have a clear purpose
2. **Type Safety**: Use TypedAgent for new implementations
3. **Error Handling**: Always handle errors gracefully
4. **Resource Cleanup**: Implement proper shutdown procedures

### Message Design

1. **Clear Types**: Define specific message types for different purposes
2. **Validation**: Validate message content early
3. **Serialization**: Ensure messages are serializable
4. **Versioning**: Plan for message format evolution

### Tool Development

1. **Argument Validation**: Implement thorough validation
2. **Error Recovery**: Handle tool failures gracefully
3. **Documentation**: Provide clear tool descriptions
4. **Testing**: Test both success and failure paths

### Memory Usage

1. **Size Limits**: Always set appropriate content limits
2. **Validation**: Validate content before storage
3. **Cleanup**: Implement memory cleanup strategies
4. **Monitoring**: Track memory usage patterns

## Troubleshooting

### Common Issues

1. **Compilation Errors**: Check feature flags and dependencies
2. **Type Mismatches**: Ensure message types match agent expectations
3. **Runtime Panics**: Check for unwrap() calls and add proper error handling
4. **Performance Issues**: Profile and identify bottlenecks

### Debugging Tips

1. **Enable Logging**: Use tracing for detailed logs
2. **Use Debugger**: Rust has excellent debugging support
3. **Check Error Context**: Error messages include detailed context
4. **Validate Inputs**: Ensure all inputs are properly validated

### Getting Help

- **Documentation**: Check the API docs and examples
- **Issues**: Search existing GitHub issues
- **Community**: Join the AutoGen community discussions
- **Contributing**: See the contribution guidelines above
