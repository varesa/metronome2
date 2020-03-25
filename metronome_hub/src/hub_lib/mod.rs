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

    pub struct SessionStatistics {
        pub last_rx: f64,
        pub last_seq: u64,
        pub received_messages: u64,
        pub holes_created: u64,
        pub holes_closed: u64,
        pub holes_timed_out: u64,
        pub holes: std::collections::HashMap<u64, Hole>,
    }

    impl SessionStatistics {
        pub fn new(seq: u64, rx_time: f64) -> SessionStatistics {
            let new_session = SessionStatistics {
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