mod datatypes {
    #[derive(Clone)]
    pub struct Statistics {
        pub sent: u64,
        pub recv: u64,
        pub lost: u64,
        pub noncontigous: u64,

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
        pub key: String,
        pub sid: String,
        pub probe_id: String,
    }
}