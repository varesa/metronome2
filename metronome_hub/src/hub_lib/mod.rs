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

    pub struct Hole {
        pub created: f64,
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
    }

    pub struct SessionContainer {
        pub last_stats: f64,
        pub last_rx: f64,
        pub last_seq: u64,
        pub received_messages: u64,
        pub holes_created: u64,
        pub holes_closed: u64,
        pub holes_timed_out: u64,
        pub holes: std::collections::HashMap<u64, Hole>,
    }

    impl ServerSessionStatistics {
        pub fn from_session_container(sid: &std::string::String, session_container: &SessionContainer) -> ServerSessionStatistics {
            return ServerSessionStatistics {
                clocktower_type: "server_session_statistics".to_string(),
                sid: sid.clone(),
                timestamp: session_container.last_rx,
                received_messages: session_container.received_messages,
                holes_created: session_container.holes_created,
                holes_closed: session_container.holes_closed,
                holes_timed_out: session_container.holes_timed_out,
                holes_current: session_container.holes.len() as u64,
            }
        }

        pub fn to_json(self) -> Result<std::string::String, serde_json::Error> {
            return serde_json::to_string(&self);
        }
    }

    impl SessionContainer {
        pub fn new(seq: u64, rx_time: f64) -> SessionContainer {
            let new_session = SessionContainer {
                last_stats: 0.0,
                last_rx: rx_time,
                last_seq: seq,
                received_messages: 1,
                holes_created: 0,
                holes_closed: 0,
                holes_timed_out: 0,
                holes: std::collections::HashMap::new(),
            };
            return new_session;
        }

        pub fn seq_analyze(&mut self, seq: u64, current_time: f64) {
            self.received_messages += 1;
            self.last_rx = current_time;
            if seq == (self.last_seq + 1) {
                self.last_seq = seq;
            } else if self.holes.contains_key(&seq) {
                self.holes_closed += 1;
                self.holes.remove(&seq);
            } else {
                let start = self.last_seq + 1;
                let end = seq - 1;
                for i in start..=end {
                    if !self.holes.contains_key(&i) {
                        self.holes.insert(i, Hole { created: current_time });
                        self.holes_created += 1;
                    }
                }
                self.last_seq = seq;
            }
        }

        pub fn prune_holes(&mut self, deadline: f64) {
            let mut remove_items: Vec<u64> = Vec::new();
            for (hole_seq, hole) in self.holes.iter() {
                if hole.created < deadline {
                    remove_items.push(*hole_seq);
                }
            }
            for remove_item in remove_items.iter() {
                self.holes_timed_out += 1;
                self.holes.remove(remove_item);
            }
        }
    }
}