mod datatypes {
    #[derive(Clone)]
    pub struct ServerStatistics {
        pub sid: String,
        pub last_rx: f64,
        pub recv: u64,
        pub noncontigous: u64,
    }

    #[derive(Clone)]
    pub struct ServerConfig {
        pub bind: std::net::SocketAddr,
        pub key: String,
    }
}