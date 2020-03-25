extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate rmp_serde;
extern crate time;

pub mod datatypes {
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
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
}

pub mod util {
    pub fn get_timestamp() -> f64 {
        let current_time_duration = time::OffsetDateTime::now() - time::OffsetDateTime::unix_epoch();
        return current_time_duration.as_seconds_f64();
    }
}