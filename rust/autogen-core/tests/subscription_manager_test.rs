use autogen_core::agent_id::AgentId;
use autogen_core::agent_type::AgentType;
use autogen_core::runtime_impl_helpers::SubscriptionManager;
use autogen_core::subscription::Subscription;
use autogen_core::subscription_manager::SubscriptionError;
use autogen_core::topic::TopicId;
use autogen_core::type_subscription::TypeSubscription;
use autogen_core::type_prefix_subscription::TypePrefixSubscription;
use std::error::Error;

#[tokio::test]
async fn test_subscription_manager_creation() {
    let manager = SubscriptionManager::new();
    assert_eq!(manager.subscriptions().len(), 0);
    
    let stats = manager.get_subscription_stats();
    assert_eq!(stats.get("total_subscriptions"), Some(&0));
    assert_eq!(stats.get("seen_topics"), Some(&0));
    assert_eq!(stats.get("cached_recipients"), Some(&0));
}

#[tokio::test]
async fn test_add_subscription_success() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypeSubscription::new(
        "test_topic".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    
    let result = manager.add_subscription(Box::new(subscription)).await;
    assert!(result.is_ok(), "Failed to add subscription: {:?}", result);
    
    assert_eq!(manager.subscriptions().len(), 1);
    
    let stats = manager.get_subscription_stats();
    assert_eq!(stats.get("total_subscriptions"), Some(&1));
}

#[tokio::test]
async fn test_add_duplicate_subscription() {
    let mut manager = SubscriptionManager::new();

    // Create a mock subscription with a fixed ID to test duplication
    struct FixedIdSubscription {
        id: String,
    }

    impl Subscription for FixedIdSubscription {
        fn id(&self) -> &str {
            &self.id
        }

        fn is_match(&self, _topic_id: &TopicId) -> bool {
            true
        }

        fn map_to_agent(&self, topic_id: &TopicId) -> Result<AgentId, Box<dyn Error>> {
            AgentId::new(
                &AgentType { r#type: "TestAgent".to_string() },
                &topic_id.source,
            ).map_err(|e| Box::new(SubscriptionError::Invalid(e)) as Box<dyn Error>)
        }
    }

    let subscription1 = FixedIdSubscription {
        id: "test_subscription_id".to_string(),
    };
    let subscription_id = subscription1.id().to_string();

    let subscription2 = FixedIdSubscription {
        id: "test_subscription_id".to_string(), // Same ID
    };

    // Add first subscription
    let result1 = manager.add_subscription(Box::new(subscription1)).await;
    assert!(result1.is_ok());

    // Try to add duplicate subscription with same ID
    let result2 = manager.add_subscription(Box::new(subscription2)).await;
    assert!(result2.is_err());

    match result2 {
        Err(SubscriptionError::AlreadyExists(id)) => {
            assert_eq!(id, subscription_id);
        }
        _ => panic!("Expected AlreadyExists error"),
    }
}

#[tokio::test]
async fn test_add_subscription_with_empty_id() {
    let mut manager = SubscriptionManager::new();
    
    // Create a mock subscription with empty ID
    struct EmptyIdSubscription;
    
    impl Subscription for EmptyIdSubscription {
        fn id(&self) -> &str {
            ""
        }
        
        fn is_match(&self, _topic_id: &TopicId) -> bool {
            false
        }
        
        fn map_to_agent(&self, _topic_id: &TopicId) -> Result<AgentId, Box<dyn Error>> {
            Err(Box::new(SubscriptionError::Invalid("Empty ID".to_string())))
        }
    }
    
    let result = manager.add_subscription(Box::new(EmptyIdSubscription)).await;
    assert!(result.is_err());
    
    match result {
        Err(SubscriptionError::Invalid(msg)) => {
            assert!(msg.contains("empty"));
        }
        _ => panic!("Expected Invalid error for empty ID"),
    }
}

#[tokio::test]
async fn test_remove_subscription_success() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypeSubscription::new(
        "test_topic".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    let subscription_id = subscription.id().to_string();
    
    // Add subscription
    manager.add_subscription(Box::new(subscription)).await.unwrap();
    assert_eq!(manager.subscriptions().len(), 1);
    
    // Remove subscription
    let result = manager.remove_subscription(&subscription_id).await;
    assert!(result.is_ok(), "Failed to remove subscription: {:?}", result);
    
    assert_eq!(manager.subscriptions().len(), 0);
}

#[tokio::test]
async fn test_remove_nonexistent_subscription() {
    let mut manager = SubscriptionManager::new();
    
    let result = manager.remove_subscription("nonexistent_id").await;
    assert!(result.is_err());
    
    match result {
        Err(SubscriptionError::NotFound(id)) => {
            assert_eq!(id, "nonexistent_id");
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_remove_subscription_with_empty_id() {
    let mut manager = SubscriptionManager::new();
    
    let result = manager.remove_subscription("").await;
    assert!(result.is_err());
    
    match result {
        Err(SubscriptionError::Invalid(msg)) => {
            assert!(msg.contains("empty"));
        }
        _ => panic!("Expected Invalid error for empty ID"),
    }
}

#[tokio::test]
async fn test_get_subscribed_recipients() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypeSubscription::new(
        "test_topic".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    
    manager.add_subscription(Box::new(subscription)).await.unwrap();
    
    let topic = TopicId {
        r#type: "test_topic".to_string(),
        source: "test_source".to_string(),
    };
    
    let recipients = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients.len(), 1);
    
    let agent_id = &recipients[0];
    assert_eq!(agent_id.r#type, "TestAgent");
    assert_eq!(agent_id.key, "test_source");
}

#[tokio::test]
async fn test_get_subscribed_recipients_no_match() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypeSubscription::new(
        "test_topic".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    
    manager.add_subscription(Box::new(subscription)).await.unwrap();
    
    let topic = TopicId {
        r#type: "different_topic".to_string(),
        source: "test_source".to_string(),
    };
    
    let recipients = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients.len(), 0);
}

#[tokio::test]
async fn test_prefix_subscription() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypePrefixSubscription::new(
        "test_".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    
    manager.add_subscription(Box::new(subscription)).await.unwrap();
    
    // Test matching topic with prefix
    let topic1 = TopicId {
        r#type: "test_topic".to_string(),
        source: "source1".to_string(),
    };
    
    let recipients1 = manager.get_subscribed_recipients(&topic1).await;
    assert_eq!(recipients1.len(), 1);
    
    // Test matching topic with different suffix
    let topic2 = TopicId {
        r#type: "test_another".to_string(),
        source: "source2".to_string(),
    };
    
    let recipients2 = manager.get_subscribed_recipients(&topic2).await;
    assert_eq!(recipients2.len(), 1);
    
    // Test non-matching topic
    let topic3 = TopicId {
        r#type: "different_topic".to_string(),
        source: "source3".to_string(),
    };
    
    let recipients3 = manager.get_subscribed_recipients(&topic3).await;
    assert_eq!(recipients3.len(), 0);
}

#[tokio::test]
async fn test_subscription_indexing() {
    let mut manager = SubscriptionManager::new();
    
    let subscription = TypeSubscription::new(
        "test_topic".to_string(),
        AgentType { r#type: "TestAgent".to_string() },
    );
    let subscription_id = subscription.id().to_string();
    
    manager.add_subscription(Box::new(subscription)).await.unwrap();
    
    // Test ID-based lookup
    let found_subscription = manager.get_subscription_by_id(&subscription_id);
    assert!(found_subscription.is_some());
    assert_eq!(found_subscription.unwrap().id(), subscription_id);
    
    // Test lookup of non-existent subscription
    let not_found = manager.get_subscription_by_id("nonexistent");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_subscription_matching() {
    let manager = SubscriptionManager::new();
    
    let topic = TopicId {
        r#type: "test_topic".to_string(),
        source: "test_source".to_string(),
    };
    
    // Test with no subscriptions
    let matching = manager.find_matching_subscriptions(&topic);
    assert_eq!(matching.len(), 0);
    
    let has_match = manager.has_matching_subscription(&topic);
    assert!(!has_match);
}

#[tokio::test]
async fn test_subscription_stats() {
    let mut manager = SubscriptionManager::new();
    
    let subscription1 = TypeSubscription::new(
        "topic1".to_string(),
        AgentType { r#type: "Agent1".to_string() },
    );
    
    let subscription2 = TypePrefixSubscription::new(
        "prefix_".to_string(),
        AgentType { r#type: "Agent2".to_string() },
    );
    
    manager.add_subscription(Box::new(subscription1)).await.unwrap();
    manager.add_subscription(Box::new(subscription2)).await.unwrap();
    
    let stats = manager.get_subscription_stats();
    assert_eq!(stats.get("total_subscriptions"), Some(&2));
    assert_eq!(stats.get("seen_topics"), Some(&0)); // No topics processed yet
    assert_eq!(stats.get("cached_recipients"), Some(&0)); // No cache entries yet
}

#[tokio::test]
async fn test_state_persistence() {
    let mut manager = SubscriptionManager::new();

    // Add some subscriptions
    let subscription1 = TypeSubscription::new(
        "topic1".to_string(),
        AgentType { r#type: "Agent1".to_string() },
    );

    let subscription2 = TypePrefixSubscription::new(
        "prefix_".to_string(),
        AgentType { r#type: "Agent2".to_string() },
    );

    manager.add_subscription(Box::new(subscription1)).await.unwrap();
    manager.add_subscription(Box::new(subscription2)).await.unwrap();

    // Process some topics to populate seen_topics
    let topic1 = TopicId {
        r#type: "topic1".to_string(),
        source: "source1".to_string(),
    };

    let topic2 = TopicId {
        r#type: "prefix_test".to_string(),
        source: "source2".to_string(),
    };

    manager.get_subscribed_recipients(&topic1).await;
    manager.get_subscribed_recipients(&topic2).await;

    // Save state
    let state = manager.save_state().unwrap();

    // Verify state structure
    assert!(state.is_object());
    let state_obj = state.as_object().unwrap();

    assert!(state_obj.contains_key("total_subscriptions"));
    assert!(state_obj.contains_key("subscriptions"));
    assert!(state_obj.contains_key("seen_topics"));
    assert!(state_obj.contains_key("version"));
    assert!(state_obj.contains_key("saved_at"));

    // Verify subscription count
    assert_eq!(state_obj.get("total_subscriptions").unwrap().as_u64(), Some(2));

    // Verify seen topics count
    assert_eq!(state_obj.get("total_seen_topics").unwrap().as_u64(), Some(2));

    // Create new manager and load state metadata
    let mut new_manager = SubscriptionManager::new();
    let result = new_manager.load_state_metadata(&state);
    assert!(result.is_ok(), "Failed to load state: {:?}", result);

    // Verify that seen topics were restored
    let new_stats = new_manager.get_subscription_stats();
    assert_eq!(new_stats.get("seen_topics"), Some(&2));
}

#[tokio::test]
async fn test_load_invalid_state() {
    let mut manager = SubscriptionManager::new();

    // Test with invalid JSON structure
    let invalid_state = serde_json::json!("not an object");
    let result = manager.load_state_metadata(&invalid_state);
    assert!(result.is_err());

    match result {
        Err(SubscriptionError::Invalid(msg)) => {
            assert!(msg.contains("JSON object"));
        }
        _ => panic!("Expected Invalid error"),
    }
}

#[tokio::test]
async fn test_multiple_subscriptions_same_topic() {
    let mut manager = SubscriptionManager::new();

    // Add multiple subscriptions for the same topic type
    let subscription1 = TypeSubscription::new(
        "shared_topic".to_string(),
        AgentType { r#type: "Agent1".to_string() },
    );

    let subscription2 = TypeSubscription::new(
        "shared_topic".to_string(),
        AgentType { r#type: "Agent2".to_string() },
    );

    manager.add_subscription(Box::new(subscription1)).await.unwrap();
    manager.add_subscription(Box::new(subscription2)).await.unwrap();

    let topic = TopicId {
        r#type: "shared_topic".to_string(),
        source: "test_source".to_string(),
    };

    let recipients = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients.len(), 2);

    // Verify both agents are present
    let agent_types: Vec<&str> = recipients.iter().map(|agent| agent.r#type.as_str()).collect();
    assert!(agent_types.contains(&"Agent1"));
    assert!(agent_types.contains(&"Agent2"));
}

#[tokio::test]
async fn test_subscription_cache_behavior() {
    let mut manager = SubscriptionManager::new();

    let subscription = TypeSubscription::new(
        "cached_topic".to_string(),
        AgentType { r#type: "CachedAgent".to_string() },
    );

    manager.add_subscription(Box::new(subscription)).await.unwrap();

    let topic = TopicId {
        r#type: "cached_topic".to_string(),
        source: "cache_source".to_string(),
    };

    // First call should build cache
    let recipients1 = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients1.len(), 1);

    // Second call should use cache
    let recipients2 = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients2.len(), 1);
    assert_eq!(recipients1, recipients2);

    // Verify cache statistics
    let stats = manager.get_subscription_stats();
    assert_eq!(stats.get("seen_topics"), Some(&1));
    assert_eq!(stats.get("cached_recipients"), Some(&1));
}

#[tokio::test]
async fn test_subscription_removal_clears_cache() {
    let mut manager = SubscriptionManager::new();

    let subscription = TypeSubscription::new(
        "removable_topic".to_string(),
        AgentType { r#type: "RemovableAgent".to_string() },
    );
    let subscription_id = subscription.id().to_string();

    manager.add_subscription(Box::new(subscription)).await.unwrap();

    let topic = TopicId {
        r#type: "removable_topic".to_string(),
        source: "remove_source".to_string(),
    };

    // Build cache
    let recipients_before = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients_before.len(), 1);

    // Remove subscription
    manager.remove_subscription(&subscription_id).await.unwrap();

    // Cache should be rebuilt and return empty results
    let recipients_after = manager.get_subscribed_recipients(&topic).await;
    assert_eq!(recipients_after.len(), 0);
}
