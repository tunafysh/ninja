use std::process::Command;
use std::io::{Error, ErrorKind};
use zbus::zvariant::{Value, OwnedObjectPath};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShurikenStatus {
    Active,
    Inactive,
    Error(String),
    Reloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shuriken {
    name: String,
    service_name: String,
    status: ShurikenStatus,
}

impl Shuriken {
    /// Create a new service component (Shuriken)
    pub fn new(name: &str, service_name: &str) -> Self {
        Shuriken {
            name: name.to_string(),
            service_name: service_name.to_string(),
            status: ShurikenStatus::Inactive,
        }
    }

    /// Throw the Shuriken (start service)
    pub async fn throw(&mut self) -> Result<(), String> {
        self.execute_service_action("start").await?;
        self.status = ShurikenStatus::Active;
        Ok(())
    }

    /// Recall the Shuriken (stop service)
    pub async fn recall(&mut self) -> Result<(), String> {
        self.execute_service_action("stop").await?;
        self.status = ShurikenStatus::Inactive;
        Ok(())
    }

    /// Spin the Shuriken (restart service)
    pub async fn spin(&mut self) -> Result<(), String> {
        self.execute_service_action("restart").await?;
        self.status = ShurikenStatus::Reloading;
        Ok(())
    }

    /// Check shuriken position (service status)
    pub async fn track(&mut self) -> Result<ShurikenStatus, String> {
        let output = self.execute_service_action("is-active").await?;
        
        self.status = match output.trim() {
            "active" => ShurikenStatus::Active,
            "inactive" => ShurikenStatus::Inactive,
            _ => ShurikenStatus::Error(output),
        };
        
        Ok(self.status.clone())
    }

    /// Internal service command execution with Polkit
    async fn execute_service_action(&self, action: &str) -> Result<String, String> {
        // Use the Polkit authentication from previous implementation
        let connection = zbus::Connection::system().await.map_err(|e| e.to_string())?;
        
        // Get the current process ID for Polkit
        let pid = std::process::id();
        
        // Prepare subject for Polkit authorization
        let subject = HashMap::from([
            ("pid", Value::U32(pid)),
            ("start-time", Value::U64(0)),
            ("subject-kind", Value::Str("unix-process".into())),
        ]);
        
        // Define the action ID for systemd service management
        let action_id = format!("org.freedesktop.systemd1.manage-units");
        
        // Polkit authorization check
        let reply = connection.call_method(
                Some("org.freedesktop.PolicyKit1"),
                "/org/freedesktop/PolicyKit1/Authority",
                Some("org.freedesktop.PolicyKit1.Authority"),
                "CheckAuthorization",
                &(
                    subject,                    // Subject (the current process)
                    action_id,                  // Action ID
                    HashMap::<String, Value>::new(), // Details
                    1u32,                       // Flags (1 = AllowUserInteraction)
                    ""                          // Cancellation ID (empty string)
                )
            )
            .await
            .map_err(|e| e.to_string())?;
        
        // Extract the response tuple from the message body
        let (is_authorized, _, _): (bool, u32, HashMap<String, Value>) = reply.body().deserialize().map_err(|e| e.to_string())?;
            
        
        if !is_authorized {
            return Err("Unauthorized service operation".into());
        }
        
        // Execute systemctl command
        let output = Command::new("systemctl")
            .arg(action)
            .arg(&self.service_name)
            .output()
            .map_err(|e| e.to_string())?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }   
    
}

// Trait for component management
pub trait ServiceController {
    fn get_name(&self) -> &str;
    fn get_status(&self) -> &ShurikenStatus;
    fn get_service_name(&self) -> &str;
}

impl ServiceController for Shuriken {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_status(&self) -> &ShurikenStatus {
        &self.status
    }

    fn get_service_name(&self) -> &str {
        &self.service_name
    }
}