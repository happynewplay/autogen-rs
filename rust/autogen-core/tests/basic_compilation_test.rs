// Basic compilation test to verify that the core types can be instantiated
use autogen_core::single_threaded_agent_runtime::RuntimeError;
use autogen_core::agent_id::AgentId;
use autogen_core::agent_type::AgentType;
use autogen_core::agent_metadata::AgentMetadata;
use autogen_core::single_threaded_agent_runtime::SingleThreadedAgentRuntime;

#[tokio::test]
async fn test_basic_types_compile() {
    // Test that basic types can be instantiated without compilation errors

    // Test AgentType
    let agent_type = AgentType {
        r#type: "test_type".to_string(),
    };

    // Test AgentId
    let agent_id = AgentId::new(&agent_type, "test_key").unwrap();
    assert_eq!(agent_id.r#type, "test_type");

    // Test AgentMetadata
    let metadata = AgentMetadata {
        r#type: "test_type".to_string(),
        key: "test_key".to_string(),
        description: "Test agent".to_string(),
    };
    assert_eq!(metadata.description, "Test agent");

    // Test that SingleThreadedAgentRuntime can be created
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    println!("All basic types compiled and instantiated successfully!");
}

#[test]
fn test_error_types() {
    // Test that error types can be created
    let error = RuntimeError::General("test error".to_string());
    assert!(error.to_string().contains("test error"));

    println!("Error types work correctly!");
}
