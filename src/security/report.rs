use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub normalized_message: Option<String>,
    pub flags: Vec<String>,
}
