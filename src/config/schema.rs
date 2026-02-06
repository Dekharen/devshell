use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub stages: Vec<String>,
    #[serde(default)]
    pub docker_args: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub host: String,
    pub container: String,
    pub mode: String,
}
