pub mod client;
pub mod config;
pub mod state;

use config::AgentConfig;

/// Initialize the agent module with configuration from a JSON file
pub fn init(config_path: &str) {
    log::info!("agent: loading config from {}", config_path);

    let json = std::fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("Failed to read agent config '{}': {}", config_path, e));

    let config: AgentConfig = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("Failed to parse agent config: {}", e));

    match client::AgentClient::new(config.socket_path.clone()) {
        Ok(client) => {
            state::init_state(client);
            log::info!("agent: initialized with socket path: {}", config.socket_path);
        }
        Err(e) => {
            log::error!("Failed to initialize agent client: {}", e);
            panic!("Failed to initialize agent client: {}", e);
        }
    }
}
