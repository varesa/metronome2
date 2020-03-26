extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate rmp_serde;
extern crate time;

pub mod datatypes {
    #[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
    pub struct MetronomeMessage {
        pub mode: String,
        pub payload: Option<String>,
        pub mul: f32,
        pub seq: u64,
        pub key: String,
        pub sid: String,
    }

    pub struct WrappedMessage {
        pub addr: std::net::SocketAddr,
        pub message: MetronomeMessage,
    }
    
    impl MetronomeMessage {
        pub fn parse_from_buffer(buffer: &[u8;65536]) -> Option<MetronomeMessage> {
            if let Ok(deserialized) = rmp_serde::from_slice::<MetronomeMessage>(buffer) {
                return Some(deserialized);
            } else {
                return None;
            }
        }

        pub fn as_vec(self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
            return rmp_serde::to_vec(&self);
        }

        pub fn get_pong(self) -> MetronomeMessage {
            let new_payload: Option<String>;

            if let Some(payload) = &self.payload {
                if self.mul != 1.0 {
                    let target_len : usize = ((payload.len() as f32) * self.mul) as usize;
                    new_payload = Some(std::iter::repeat(payload.chars().next().unwrap()).take(target_len).collect::<String>());
                } else {
                    new_payload = self.payload;
                }
            } else {
                new_payload = None;
            }

            let reply_message = MetronomeMessage {
                mode: "pong".to_string(),
                payload: new_payload,
                mul: self.mul,
                seq: self.seq,
                key: self.key,
                sid: self.sid,
            };

            return reply_message;
        }
    }

    pub struct Hole {
        pub created: f64,
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

pub mod util {
    pub fn get_timestamp() -> f64 {
        let current_time_duration = time::OffsetDateTime::now() - time::OffsetDateTime::unix_epoch();
        return current_time_duration.as_seconds_f64();
    }
}