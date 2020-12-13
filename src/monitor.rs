use std::io::Read;
use std::io::Write;

/// Configuration for sending protocol monitoring data
#[derive(Clone)]
pub struct MonitoringConfig {
    /// Enable sending data
    enabled: bool,
    /// Monitoring host
    host: String,
    /// Peer URL endpoint
    peer_path: String,
    /// Updates URL endpoint
    update_path: String,
}

impl MonitoringConfig {
    /// Creates a new monitoring configuration
    ///
    /// # Arguments
    ///
    /// * `enabled` - Share monitoring data
    /// * `url` - URL of monitoring host
    pub fn new(enabled: bool, host: String, peer_path: String, update_path: String) -> MonitoringConfig {
        MonitoringConfig {
            enabled,
            host,
            peer_path,
            update_path,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Send monitoring data of peers
    ///
    /// # Arguments
    ///
    /// * `pid` - Identifier of sending process
    /// * `peers` - List of peers in the view of the process
    pub fn send_peer_data(&self, pid: String, peers: Vec<String>) {
        let host = self.host.clone();
        let path = self.peer_path.clone();
        std::thread::spawn(move || {
            let peers_str = peers.iter()
                .map(|peer| format!("\"{}\"", peer))
                .collect::<Vec<String>>().join(",");
            let json = format!("{{\
                \"id\":\"{}\",\
                \"peers\":[{}],\
                \"messages\":[{}]\
            }}", pid, peers_str, "");
            log::trace!("send_data:\n{}", json);
            match MonitoringConfig::post(&host, &path, json) {
                Ok(()) => log::trace!("Peer {}: peer monitoring data sent", pid),
                Err(e) => log::warn!("Peer {} peer could not send monitoring data to {}: {}", pid, host, e),
            }
        });
    }

    /// Send monitoring data of updates
    ///
    /// # Arguments
    ///
    /// * `pid` - Identifier of sending process
    /// * `updates` - List of updates the process has received
    pub fn send_update_data(&self, pid: String, updates: Vec<String>) {
        let host = self.host.clone();
        let path = self.peer_path.clone();
        std::thread::spawn(move || {
            let updates_str = updates.iter()
                .map(|update| format!("\"{}\"", update))
                .collect::<Vec<String>>().join(",");
            let json = format!("{{\
                \"id\":\"{}\",\
                \"peers\":[{}],\
                \"messages\":[{}]\
            }}", pid, "", updates_str);
            log::trace!("send_data:\n{}", json);
            match MonitoringConfig::post(&host, &path, json) {
                Ok(()) => log::trace!("Peer {}: update monitoring data sent", pid),
                Err(e) => log::warn!("Peer {} could not send update monitoring data to {}: {}", pid, host, e),
            }
        });
    }

    fn post(host: &str, path: &str, json: String) -> std::io::Result<()> {

        let bytes = json.as_bytes();

        let mut stream = std::net::TcpStream::connect(host)?;

        let mut request_data = String::new();
        request_data.push_str(&format!("POST {} HTTP/1.1", path));
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
            peer_path: "".to_string(),
            update_path: "".to_string(),
        }
    }
}
