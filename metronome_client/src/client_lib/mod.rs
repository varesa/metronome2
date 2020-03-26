pub mod datatypes {
    #[derive(Clone)]
    pub struct ClientSessionStatistics {
        pub clocktower_type: std::string::String,

        pub sid: std::string::String,
        pub timestamp: f64,
        pub sent_messages: u64,
        pub received_messages: u64,
        pub unexpected_seq: u64,
        pub packets_lost: u64,
        pub packets_inflight: u64,

        pub rtt_worst: f64,
        pub rtt_best: f64,
        pub rtt_mavg: f64,
    }

    pub struct ClientSessionTracker {
        pub last_rx: Option<f64>,
        pub last_tx: Option<f64>,
        
        pub last_rx_seq: Option<u64>,
        pub next_expected_seq: u64,
        pub max_seq: u64,

        pub seq_unexpected_increment: u64,
        pub seq_unexpected_decrement: u64,

        pub sent_messages: u64,
        pub received_messages: u64,
        pub timely_received_messages: u64,
        
        pub lost_messages: u64,
        pub inflight_messages: u64,

        pub rtt_worst: Option<f64>,
        pub rtt_best: Option<f64>,
        pub rtt_mavg: Option<f64>,
    }

    impl ClientSessionTracker {
        pub fn new() -> ClientSessionTracker {
            return ClientSessionTracker {
                last_rx: None,
                last_tx: None,
                
                last_rx_seq: None,
                next_expected_seq: 0,
                max_seq: 0,
                seq_unexpected_increment: 0,
                seq_unexpected_decrement: 0,

                sent_messages: 0,
                received_messages: 0,
                timely_received_messages: 0,

                lost_messages: 0,
                inflight_messages: 0,

                rtt_worst: None,
                rtt_best: None,
                rtt_mavg: None,
            };
        }

        pub fn outgoing(&mut self, timestamp: f64) {
            self.last_tx = Some(timestamp);
            self.sent_messages += 1;
            self.inflight_messages += 1;
        }

        pub fn incoming(&mut self, timestamp: f64, seq: u64,) {
            if let Some(last_rx_seq) = self.last_rx_seq {
                if seq == self.next_expected_seq {
                    // All good, we are receiving the frame we though we were going to get
                } else if seq > self.next_expected_seq && seq < (self.max_seq + 1) {
                    // If sequence number is greater than next expected but smaller or equal than maximum seen
                    // we can assume we are seeing reordered messages
                    self.seq_unexpected_increment += 1;
                } else if seq < self.next_expected_seq {
                    // If sequence number is smaller than we expect, assume we are seeing reordered (older) frames
                    self.seq_unexpected_decrement += 1;
                }
            } else {
                self.last_rx_seq = Some(seq);
            }
            self.received_messages += 1;
            self.next_expected_seq = seq + 1;
            self.last_rx_seq = Some(seq);
            self.max_seq = self.max_seq.max(seq);
        }

        pub fn rtt_timeout(&mut self) {
            self.inflight_messages -= 1;
            self.lost_messages += 1;
        }

        pub fn rtt_success(&mut self) {
            self.inflight_messages -= 1;
            self.timely_received_messages += 1;
        }
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

    pub struct RTTMeasurement {
        pub seq: u64,
        pub timestamp: f64,
    }
}