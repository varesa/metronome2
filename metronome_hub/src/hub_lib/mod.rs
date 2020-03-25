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
        pub serialized_message: Vec<u8>
    }
}