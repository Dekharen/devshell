use serde::{Deserialize, Serialize};

use crate::error::DevshellError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum User {
    #[serde(rename = "host_mirror")]
    HostMirror {
        name: String,
        proxy: String,
        home: String,
    },
    #[serde(rename = "container_local")]
    ContainerLocal {
        name: String,
        home: Option<String>,
        #[serde(default)]
        shell: Option<String>,
    },
    #[serde(rename = "named_existing")]
    NamedExisting { name: String, home: Option<String> },
    #[serde(rename = "root")]
    Root {},
}

impl User {
    pub fn name(&self) -> &str {
        match self {
            User::HostMirror { name, .. } => name,
            User::ContainerLocal { name, .. } => name,
            User::NamedExisting { name, .. } => name,
            User::Root {} => "root",
        }
    }

    pub fn as_host_mirror(&self) -> Option<(&str, &str, &str)> {
        match self {
            User::HostMirror { name, proxy, home } => Some((name, proxy, home)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntry {
    #[serde(flatten)]
    pub user: User,
    #[serde(default)]
    pub default: bool,
}

impl UserEntry {
    pub fn name(&self) -> &str {
        self.user.name()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub stages: Vec<String>,
    #[serde(default)]
    pub docker_args: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
    pub attach_command: Option<String>,
    #[serde(default)]
    pub users: Vec<UserEntry>,
}

impl Config {
    pub fn get_user(&self, name: &str) -> Option<&UserEntry> {
        self.users.iter().find(|u| u.name() == name)
    }

    pub fn get_default_user(&self) -> Option<&UserEntry> {
        self.users.iter().find(|u| u.default)
    }

    pub fn validate_users(&self) -> Result<(), DevshellError> {
        if self.users.is_empty() {
            return Err(DevshellError::NoUserDeclared);
        }

        let default_count = self.users.iter().filter(|u| u.default).count();
        if default_count == 0 {
            return Err(DevshellError::NoDefaultUser);
        }
        if default_count > 1 {
            return Err(DevshellError::MultipleDefaultUsers);
        }

        Ok(())
    }

    pub fn find_user_or_default(&self, name: Option<&str>) -> Result<&UserEntry, DevshellError> {
        if let Some(name) = name {
            self.get_user(name)
                .ok_or_else(|| DevshellError::UserNotFound {
                    requested: name.to_string(),
                    suggestion: format!(
                        "To connect as user '{}', add it to your devshell config:\n\n\
[[user]]\nkind = \"container_local\"\nname = \"{}\"\n\n\
Or for host mirroring:\n\n\
[[user]]\nkind = \"host_mirror\"\nname = \"{}\"\nproxy = \"dev\"\nhome = \"/home/dev\"",
                        name, name, name
                    ),
                })
        } else {
            self.get_default_user()
                .ok_or_else(|| DevshellError::NoDefaultUser)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub host: String,
    pub container: String,
    pub mode: String,
}
