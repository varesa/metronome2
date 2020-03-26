pub mod datatypes {
    #[derive(Clone)]
    pub struct ClientSessionStatistics {
        pub clocktower_type: std::string::String,

        pub sid: std::string::String,
        pub timestamp: f64,
        pub received_messages: u64,
        pub holes_created: u64,
        pub holes_closed: u64,
        pub holes_timed_out: u64,
        pub holes_current: u64,

        pub rtt_worst: f64,
        pub rtt_best: f64,
        pub rtt_mavg: f64,
    }

    #[derive(Clone)]
    pub struct ClientConfig {
        pub pps_limit: u64,
        pub payload_size: usize,
        pub use_sleep: bool,
        pub balance: f32,
        pub remote: std::net::SocketAddr,
        pub clocktower: std::net::SocketAddr,
        pub key: String,
        pub sid: String,
    }
}