use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const WEBSERVER_ADDRESS: &str = "WEBSERVER_ADDRESS";
const WEBSERVER_ADDRESS_DEFAULT_PORT: u16 = 3000;
const WEBSERVER_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    WEBSERVER_ADDRESS_DEFAULT_PORT,
));

fn webserver_address() -> SocketAddr {
    std::env::var(WEBSERVER_ADDRESS)
        .ok()
        .and_then(|st| {
            st.parse()
                .map_err(|err| {
                    tracing::warn!(?err, "Failed to parse WEBSERVER_ADDRESS env var");
                    err
                })
                .ok()
        })
        .unwrap_or(WEBSERVER_ADDRESS_DEFAULT)
}

const REDIS_HOST: &str = "REDIS_HOST";
const REDIS_HOST_DEFAULT: &str = "127.0.0.1";

fn redis_host() -> String {
    std::env::var(REDIS_HOST)
        .ok()
        .unwrap_or_else(|| REDIS_HOST_DEFAULT.to_owned())
}

const REDIS_PORT: &str = "REDIS_PORT";
const REDIS_PORT_DEFAULT: u16 = 6379;

fn redis_port() -> u16 {
    std::env::var(REDIS_PORT)
        .ok()
        .and_then(|st| {
            st.parse()
                .map_err(|err| {
                    tracing::warn!(?err, "Failed to parse REDIS_PORT env var");
                    err
                })
                .ok()
        })
        .unwrap_or(REDIS_PORT_DEFAULT)
}

const DATABASE_URL: &str = "DATABASE_URL";

#[track_caller]
fn database_url() -> String {
    std::env::var(DATABASE_URL).ok().unwrap_or_else(|| {
        panic!("DATABASE_URL env var not set");
    })
}

const CONFIG_FILE_PATH: &str = "CONFIG_FILE_PATH";

fn config_file_path() -> Option<PathBuf> {
    std::env::var(CONFIG_FILE_PATH).ok().map(PathBuf::from)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "webserver_address")]
    webserver_address: SocketAddr,
    #[serde(default = "redis_host")]
    redis_host: String,
    #[serde(default = "redis_port")]
    redis_port: u16,
    #[serde(default = "database_url")]
    database_url: String,
    #[serde(default = "config_file_path")]
    config_file_path: Option<PathBuf>,
}

impl Config {
    #[track_caller]
    pub fn load_from_env() -> Self {
        let config_file_path = config_file_path()
            .expect("CONFIG_FILE_PATH env var not set")
            .canonicalize()
            .expect("Failed to canonicalize config file path");
        let st = std::fs::read_to_string(config_file_path).expect("Failed to read config file");
        toml::from_str(&st).expect("Failed to parse config file")
    }

    pub fn diff(&self, other: &Self) -> toml::map::Map<String, toml::Value> {
        let mut map = toml::map::Map::new();

        macro_rules! diff {
            ($field:ident) => {
                if self.$field != other.$field {
                    map.insert(
                        stringify!($field).to_owned(),
                        toml::Value::try_from(&self.$field).unwrap(),
                    );
                }
            };
            // handle a list of fields
            ($($field:ident),*) => {
                $(diff!($field);)*
            };
        }

        diff!(webserver_address, redis_host, redis_port, database_url);

        map
    }

    pub fn webserver_address(&self) -> SocketAddr {
        self.webserver_address
    }

    pub fn redis_url(&self) -> String {
        let Self {
            redis_host,
            redis_port,
            ..
        } = self;
        format!("redis://{redis_host}:{redis_port}")
    }

    pub fn database_url(&self) -> String {
        self.database_url.clone()
    }

    pub fn config_file_path(&self) -> Option<&Path> {
        self.config_file_path.as_ref().map(|p| p.as_ref())
    }
}
