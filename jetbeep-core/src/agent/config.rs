use serde::{Deserialize, Serialize};

/// Agent configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unix socket path for agent communication
    pub socket_path: String,
}
