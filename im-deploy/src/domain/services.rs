use crate::domain::connection::ConnectionStrategy;
use crate::errors::Result;
use std::fmt;
/// Information about a deployed service
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub notes: Option<String>,
}
impl ServiceInfo {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            url: None,
            username: None,
            password: None,
            notes: None,
        }
    }
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }
    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }
    pub fn with_note(mut self, note: String) -> Self {
        self.notes = Some(note);
        self
    }
}
impl fmt::Display for ServiceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.name)?;
        if let Some(url) = &self.url {
            writeln!(f, "  URL:      {}", url)?;
        } else {
            writeln!(f, "  URL:      Not available")?;
        }
        if let Some(username) = &self.username {
            writeln!(f, "  Username: {}", username)?;
        }
        if let Some(password) = &self.password {
            writeln!(f, "  Password: {}", password)?;
        }
        if self.username.is_none() && self.password.is_none() {
            writeln!(f, "  Auth:     None")?;
        }
        if let Some(notes) = &self.notes {
            writeln!(f, "  Notes:    {}", notes)?;
        }
        Ok(())
    }
}
/// Helper to execute kubectl commands via SSH
pub fn execute_kubectl_command(
    strategy: &ConnectionStrategy,
    command: &str,
) -> Result<String> {
    let full_command = format!("sudo kubectl {}", command);
    let output = strategy.execute_command(&full_command)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
/// Get secret value from kubernetes
pub fn get_k8s_secret(
    strategy: &ConnectionStrategy,
    secret_name: &str,
    namespace: &str,
    key: &str,
) -> Result<String> {
    let command = format!(
        r#"get secret {} -n {} -o jsonpath="{{.data.{}}}" 2>/dev/null | base64 -d"#,
        secret_name, namespace, key
    );
    let output = execute_kubectl_command(strategy, &command)?;
    Ok(output.trim().to_string())
}
