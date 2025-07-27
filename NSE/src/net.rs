#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod net_api {
    use rquickjs::{Ctx, Result};
    use std::net::{TcpListener, TcpStream, ToSocketAddrs};
    use std::time::{Duration, Instant};
    use std::thread;
    use reqwest;

    /// Check if a port is available (not in use)
    #[rquickjs::function]
    pub fn port_available(port: u16) -> bool {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Check if a port is in use
    #[rquickjs::function]
    pub fn port_in_use(port: u16) -> bool {
        !port_available(port)
    }

    /// Wait for a port to become available, with timeout in milliseconds
    #[rquickjs::function]
    pub fn wait_for_port(host: String, port: u16, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        while start.elapsed() < timeout {
            if let Ok(_) = TcpStream::connect(format!("{}:{}", host, port)) {
                return Ok(true);
            }
            thread::sleep(Duration::from_millis(100));
        }
        
        Ok(false)
    }

    /// Check if we can connect to a host and port
    #[rquickjs::function]
    pub fn can_connect(host: String, port: u16) -> bool {
        match TcpStream::connect(format!("{}:{}", host, port)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Ping a host (simple TCP connection test)
    #[rquickjs::function]
    pub fn ping(host: String) -> bool {
        // Try common ports for basic connectivity
        let ports = [80, 443, 22, 21];
        
        for &port in &ports {
            if can_connect(host.clone(), port) {
                return true;
            }
        }
        false
    }

    /// Check internet connectivity by trying to connect to reliable hosts
    #[rquickjs::function]
    pub fn has_internet() -> bool {
        let test_hosts = [
            ("8.8.8.8", 53),      // Google DNS
            ("1.1.1.1", 53),      // Cloudflare DNS
            ("google.com", 80),   // Google HTTP
            ("cloudflare.com", 80) // Cloudflare HTTP
        ];
        
        for (host, port) in test_hosts {
            if can_connect(host.to_string(), port) {
                return true;
            }
        }
        false
    }

    /// Test connection speed to a host (returns time in milliseconds)
    #[rquickjs::function]
    pub fn connection_time(host: String, port: u16) -> Result<u64> {
        let start = Instant::now();
        
        match TcpStream::connect(format!("{}:{}", host, port)) {
            Ok(_) => Ok(start.elapsed().as_millis() as u64),
            Err(e) => Err(rquickjs::Error::new_from_js_message("net", "engine", format!("Connection failed: {}", e))),
        }
    }

    /// Check if a specific service is running on localhost
    #[rquickjs::function]
    pub fn service_running(port: u16) -> bool {
        can_connect("127.0.0.1".to_string(), port)
    }
    /// Find available ports in a range
    #[rquickjs::function]
    pub fn find_available_ports(ctx: Ctx<'_>, start: u16, end: u16, count: usize) -> Result<rquickjs::Array<'_>> {
        let array = rquickjs::Array::new(ctx)?;
        let mut found = 0;
        
        for port in start..=end {
            if found >= count {
                break;
            }
            if port_available(port) {
                array.set(found, port)?;
                found += 1;
            }
        }
        
        Ok(array)
    }

    /// Check if we can resolve a hostname
    #[rquickjs::function]
    pub fn can_resolve(hostname: String) -> bool {
        match format!("{}:80", hostname).to_socket_addrs() {
            Ok(mut addrs) => addrs.next().is_some(),
            Err(_) => false,
        }
    }

    /// Check if localhost is accessible
    #[rquickjs::function]
    pub fn localhost_accessible() -> bool {
        can_connect("127.0.0.1".to_string(), 80) || 
        can_connect("localhost".to_string(), 80)
    }

    /// Test if a URL is reachable (basic HTTP connectivity test)
    #[rquickjs::function]
    pub fn url_reachable(url: String) -> bool {
        // Extract host and port from URL
        if let Some(host_part) = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")) {
            let is_https = url.starts_with("https://");
            let default_port = if is_https { 443 } else { 80 };
            
            let host = host_part.split('/').next().unwrap_or("");
            let (hostname, port) = if host.contains(':') {
                let parts: Vec<&str> = host.split(':').collect();
                (parts[0].to_string(), parts[1].parse().unwrap_or(default_port))
            } else {
                (host.to_string(), default_port)
            };
            
            can_connect(hostname, port)
        } else {
            false
        }
    }

    /// Check network interface availability
    #[rquickjs::function]
    pub fn interface_up() -> bool {
        // Simple check by trying to bind to any available address
        TcpListener::bind("0.0.0.0:0").is_ok()
    }

    #[rquickjs::function]
    pub async fn fetch(url: String) -> Result<String> {
        let response = reqwest::get(url).await.expect("Failed to fetch data from link");
        let body = response.text().await.expect("Failed to get text from response");
        Ok(body)
    }
}