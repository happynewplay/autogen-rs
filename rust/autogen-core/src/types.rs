use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub id: String,
    // JSON args
    pub arguments: String,
    // Function to call
    pub name: String,
}