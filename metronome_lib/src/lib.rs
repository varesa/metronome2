#[macro_use] extern crate serde_derive;

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
}