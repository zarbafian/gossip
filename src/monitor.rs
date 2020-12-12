use std::io::Read;
use std::io::Write;

/// Configuration for sending protocol monitoring data
#[derive(Clone)]
pub struct MonitoringConfig {
    /// Enable sending data
    enabled: bool,
    /// Monitoring host
    host: String,
    /// URL context
    context: String,
}

impl MonitoringConfig {
    /// Creates a new monitoring configuration
    ///
    /// # Arguments
    ///
    /// * `enabled` - Share monitoring data
    /// * `url` - URL of monitoring host
    pub fn new(enabled: bool, url: &str) -> MonitoringConfig {
        // remove leading protocol
        let protocol_removed = if url.starts_with("http://") {
            &url[7..] }
        else {
            url
        };
        // separate host and context
        let (host, context) = match protocol_removed.find("/") {
            Some(index) => (&protocol_removed[..index], &protocol_removed[index..]),
            None => (url, "/")
        };
        MonitoringConfig {
            enabled,
            host: host.to_owned(),
            context: context.to_owned(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Send monitoring data
    ///
    /// # Arguments
    ///
    /// * `pid` - Identifier of sending process
    /// * `peers` - List of peers in the view of the process
    pub fn send_data(&self, pid: &str, peers: Vec<String>) {
        let pid = pid.to_owned();
        let host = self.host.clone();
        let context = self.context.clone();
        std::thread::spawn(move || {
            let peers_str = peers.iter()
                .map(|peer| format!("\"{}\"", peer))
                .collect::<Vec<String>>().join(",");
            let json = format!(
                "{{\
                \"id\":\"{}\",\
                \"peers\":[{}],\
                \"messages\":[{}]\
            }}", pid, peers_str, "");
            //println!("send_data:\n{}", json);
            match MonitoringConfig::post(&host, &context, json) {
                Ok(()) => log::debug!("Peer {}: monitoring data sent", pid),
                Err(e) => log::warn!("Peer {} could not send monitoring data to {}: {}", pid, host, e),
            }
        });
    }

    fn post(host: &str, context: &str, json: String) -> std::io::Result<()> {

        let bytes = json.as_bytes();

        let mut stream = std::net::TcpStream::connect(host)?;

        let mut request_data = String::new();
        request_data.push_str(&format!("POST {} HTTP/1.1", context));
        request_data.push_str("\r\n");
        request_data.push_str(&format!("Host: {}", host));
        request_data.push_str("\r\n");
        request_data.push_str("Accept: */*");
        request_data.push_str("\r\n");
        request_data.push_str("Content-Type: application/json; charset=UTF-8");
        request_data.push_str("\r\n");
        request_data.push_str(&format!("Content-Length: {}", bytes.len()));
        request_data.push_str("\r\n");
        request_data.push_str("Connection: close");
        request_data.push_str("\r\n");
        request_data.push_str("\r\n");
        request_data.push_str(&json);

        //println!("request_data = {:?}", request_data);

        let _request = stream.write_all(request_data.as_bytes())?;
        //println!("request = {:?}", request);

        let mut buf = String::new();
        let _result = stream.read_to_string(&mut buf)?;
        //println!("result = {}", result);
        log::debug!("buf = {}", buf);

        Ok(())
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        MonitoringConfig {
            enabled: false,
            host: "".to_string(),
            context: "".to_string(),
        }
    }
}
