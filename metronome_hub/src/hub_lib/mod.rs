extern crate serde_json;

pub mod datatypes {
    #[derive(Clone)]
    pub struct HubStatistics {
        pub sid: String,
        pub last_rx: f64,
        pub recv: u64,
        pub noncontigous: u64,
    }

    #[derive(Clone)]
    pub struct ServerConfig {
        pub bind: std::net::SocketAddr,
        pub key: String,
        pub clocktower: std::net::SocketAddr,
    }

    pub struct WrappedSerializedMessage {
        pub addr: std::net::SocketAddr,
        pub serialized_message: Vec<u8>,
    }

    #[derive(Serialize)]
    pub struct ServerSessionStatistics {
        pub clocktower_type: std::string::String,

        pub sid: std::string::String,
        pub timestamp: f64,
        pub received_messages: u64,
        pub holes_created: u64,
        pub holes_closed: u64,
        pub holes_timed_out: u64,
        pub holes_current: u64,

        pub received_bytes: u64,
    }

    impl ServerSessionStatistics {
        pub fn from_session_container(sid: &std::string::String, session_container: &metronome_lib::datatypes::SessionContainer) -> ServerSessionStatistics {
            return ServerSessionStatistics {
                clocktower_type: "server_session_statistics".to_string(),
                sid: sid.clone(),
                timestamp: session_container.last_rx,
                received_messages: session_container.received_messages,
                holes_created: session_container.holes_created,
                holes_closed: session_container.holes_closed,
                holes_timed_out: session_container.holes_timed_out,
                holes_current: session_container.holes.len() as u64,

                received_bytes: session_container.received_bytes,
            }
        }

        pub fn to_json(self) -> Result<std::string::String, serde_json::Error> {
            return serde_json::to_string(&self);
        }
    }
}