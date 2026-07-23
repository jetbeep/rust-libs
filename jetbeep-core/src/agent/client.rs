use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;
use crate::workq::{post_to_main, submit_bg};
use futures::channel::oneshot;
use std::time::Duration;


/// AspScriptType enum definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspScriptType {
    None = 0,
    Ui = 1,
    Updater = 2,
    Service = 3,
    Screen = 4,
    Unrecognized = -1,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ScriptTypeOrString {
    Type(AspScriptType),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HttpRequestQuery {
    #[serde(rename = "scriptType")]
    pub script_type: Option<ScriptTypeOrString>,
    #[serde(rename = "requestType")]
    pub request_type: Option<i32>,
    pub content: Option<serde_json::Value>,
}

impl HttpRequestQuery {
    /// Create a new HttpRequestQuery with the given content
    pub fn new(
        content: serde_json::Value,
        script_type: Option<ScriptTypeOrString>,
        request_type: Option<i32>,
    ) -> Self {
        Self {
            content: Some(content),
            script_type: script_type,
            request_type: request_type,
        }
    }

    /// Set the script type
    pub fn with_script_type(mut self, script_type: ScriptTypeOrString) -> Self {
        self.script_type = Some(script_type);
        self
    }

    /// Set the request type
    pub fn with_request_type(mut self, request_type: i32) -> Self {
        self.request_type = Some(request_type);
        self
    }

}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AjaxRequestData {
    pub method: String,
    pub path: String,
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AjaxResponseData<T = serde_json::Value> {
    pub status_code: i16,
    pub data: T,
}

fn parse_http_response_body<R: DeserializeOwned>(
    status_code: i16,
    body: &[u8],
    socket_path: &str,
) -> Result<AjaxResponseData<R>, String> {
    if body.is_empty() {
        let data = serde_json::from_value::<R>(serde_json::Value::Null).map_err(|e| {
            log::error!(
                "Failed to decode empty response payload as null for socket {}: {}",
                socket_path,
                e
            );
            format!("Failed to decode empty response payload: {}", e)
        })?;
        return Ok(AjaxResponseData { status_code, data });
    }

    let json_value: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
        log::error!("Failed to parse response JSON from socket {}: {}", socket_path, e);
        format!("Failed to parse response JSON: {}", e)
    })?;

    if let Ok(envelope) = serde_json::from_value::<AjaxResponseData<R>>(json_value.clone()) {
        return Ok(envelope);
    }

    let data = serde_json::from_value::<R>(json_value).map_err(|e| {
        log::error!(
            "Failed to decode response body as raw payload from socket {}: {}",
            socket_path,
            e
        );
        format!("Failed to decode response payload: {}", e)
    })?;

    Ok(AjaxResponseData { status_code, data })
}

/// Unix socket HTTP client for making requests to a specific socket
pub struct AgentClient {
    socket_path: String,
    client: reqwest::blocking::Client,
}



impl AgentClient {
    /// Create a new Unix socket client
    pub fn new(socket_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .unix_socket(socket_path.clone())
            .build()
            .map_err(|e| {
                log::error!("Failed to build HTTP client for socket {}: {}", socket_path, e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

        Ok(AgentClient { socket_path, client })
    }

    pub async fn http_request<R: DeserializeOwned + Send + 'static>(
        &self,
        query: &HttpRequestQuery,
        acceptable_codes: Option<&[i16]>,
        timeout_ms: Option<i32>,
    ) -> Result<R, Box<dyn std::error::Error>> {
        let default_codes: &[i16] = &[200, 201];
        let codes = acceptable_codes.unwrap_or(default_codes);
        let timeout = timeout_ms
            .filter(|v| *v > 0)
            .map(|v| Duration::from_millis(v as u64));
        let response: AjaxResponseData<R> = self
            .post("/sdk/http-request", &serde_json::to_value(query)?, timeout)
            .await?;
        if !codes.contains(&response.status_code) {
            return Err(format!("Unacceptable status code: {}", response.status_code).into());
        }
        Ok(response.data)
    }

    /// Get the socket path this client is connected to
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    fn build_url(&self, endpoint: &str) -> String {
        let base = Url::parse("http://localhost").unwrap();
        return base.join(endpoint).unwrap().to_string();
    }

    /// Send a POST request to the Unix socket with JSON data (async, runs on background workq)
    async fn post<R: DeserializeOwned + Send + 'static>(
        &self,
        endpoint: &str,
        json_data: &serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<AjaxResponseData<R>, Box<dyn std::error::Error>> {
        let url = self.build_url(endpoint);
        let client = self.client.clone();
        let socket_path = self.socket_path.clone();
        let json_data = json_data.clone();

        let (sender, receiver) = oneshot::channel::<Result<AjaxResponseData<R>, String>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let result = (|| {
                    let mut request_builder = client.post(&url).json(&json_data);
                    if let Some(timeout) = timeout {
                        request_builder = request_builder.timeout(timeout);
                    }

                    let response = request_builder.send().map_err(|e| {
                        log::error!("POST request failed on socket {}: {}", socket_path, e);
                        format!("POST request failed: {}", e)
                    })?;

                    let status = response.status();
                    if !status.is_success() {
                        log::error!("HTTP response status {} from agent socket {}", status, socket_path);
                        return Err(format!("Agent request error: {}", status));
                    }

                    let status_code = status.as_u16() as i16;
                    let body = response.bytes().map_err(|e| {
                        log::error!("Failed to read response body from socket {}: {}", socket_path, e);
                        format!("Failed to read response body: {}", e)
                    })?;
                    parse_http_response_body(status_code, body.as_ref(), &socket_path)
                })();

                // Bounce the result back to the main (UI) thread so the
                // awaiting task's waker is fired on the same thread that ran
                // `executor::run`. We must use `post_to_main` (cross-thread
                // safe) and NOT `submit`, because `submit` writes to a
                // thread-local queue and would silently push onto the bg
                // thread's queue — where nobody would ever drain it, leaving
                // the future hung even though the HTTP response had arrived.
                post_to_main(move || {
                    sender.send(result).ok();
                });
            });
        }

        receiver
            .await
            .map_err(|_| {
                log::error!("Failed to receive POST response from background worker");
                "Failed to receive POST response from background worker"
            })?
            .map_err(|e| e.into())
    }

    /// Send a GET request to the Unix socket (async, runs on background workq)
    async fn get<R: DeserializeOwned + Send + 'static>(&self, endpoint: &str) -> Result<AjaxResponseData<R>, Box<dyn std::error::Error>> {
        let url = self.build_url(endpoint);
        let client = self.client.clone();
        let socket_path = self.socket_path.clone();

        let (sender, receiver) = oneshot::channel::<Result<AjaxResponseData<R>, String>>();

        unsafe {
            submit_bg(Duration::from_millis(0), move |_| {
                let result = (|| {
                    let response = client
                        .get(&url)
                        .send()
                        .map_err(|e| {
                            log::error!("GET request failed on socket {}: {}", socket_path, e);
                            format!("GET request failed: {}", e)
                        })?;

                    let status = response.status();
                    if !status.is_success() {
                        log::error!("HTTP response status {} from socket {}", status, socket_path);
                        return Err(format!("HTTP error: {}", status));
                    }

                    let status_code = status.as_u16() as i16;
                    let body = response.bytes().map_err(|e| {
                        log::error!("Failed to read response body from socket {}: {}", socket_path, e);
                        format!("Failed to read response body: {}", e)
                    })?;
                    parse_http_response_body(status_code, body.as_ref(), &socket_path)
                })();

                // See post comment above for why this MUST be `post_to_main`
                // and not `submit` (the bg thread cannot reach the main
                // thread-local workq directly).
                post_to_main(move || {
                    sender.send(result).ok();
                });
            });
        }

        receiver
            .await
            .map_err(|_| "Failed to receive GET response from background worker")?
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_response_body_accepts_empty_payload_as_null_data() {
        let parsed =
            parse_http_response_body::<serde_json::Value>(201, b"", "/tmp/test-agent.sock")
                .expect("empty success body should be treated as null payload");
        assert_eq!(parsed.status_code, 201);
        assert_eq!(parsed.data, serde_json::Value::Null);
    }

    #[test]
    fn parse_http_response_body_accepts_raw_json_without_envelope() {
        let parsed = parse_http_response_body::<serde_json::Value>(
            200,
            br#"{"cmdId":20,"cmd":"openLock"}"#,
            "/tmp/test-agent.sock",
        )
        .expect("raw JSON body should be accepted");
        assert_eq!(parsed.status_code, 200);
        assert_eq!(parsed.data["cmdId"], 20);
        assert_eq!(parsed.data["cmd"], "openLock");
    }

    #[test]
    fn parse_http_response_body_keeps_explicit_envelope() {
        let parsed = parse_http_response_body::<serde_json::Value>(
            201,
            br#"{"status_code":200,"data":{"ok":true}}"#,
            "/tmp/test-agent.sock",
        )
        .expect("explicit envelope should be parsed as-is");
        assert_eq!(parsed.status_code, 200);
        assert_eq!(parsed.data["ok"], true);
    }
}
