use super::client::AgentClient;
use std::sync::{Arc, Mutex};

/// Global agent state
static AGENT_STATE: once_cell::sync::Lazy<Mutex<Option<Arc<AgentClient>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// Initialize the agent state with a client
pub fn init_state(client: AgentClient) {
    let mut state = AGENT_STATE.lock().unwrap();
    *state = Some(Arc::new(client));
    log::info!("Agent state initialized");
}

/// Get a reference to the agent client
pub fn get_client() -> Option<Arc<AgentClient>> {
    let state = AGENT_STATE.lock().unwrap();
    state.as_ref().map(Arc::clone)
}

/// Check if agent is initialized
pub fn is_initialized() -> bool {
    AGENT_STATE.lock().unwrap().is_some()
}
