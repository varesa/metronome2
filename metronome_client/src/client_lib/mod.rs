pub mod datatypes {
    #[derive(Serialize)]
    pub struct ClientSessionStatistics {
        pub clocktower_type: std::string::String,

        pub sid: std::string::String,
        pub timestamp: f64,

        pub seq_unexpected_increment: u64,
        pub seq_unexpected_decrement: u64,

        pub sent_messages: u64,
        pub received_messages: u64,
        pub timely_received_messages: u64,
        
        pub lost_messages: u64,
        pub inflight_messages: u64,

        #[serde(skip_serializing_if="Option::is_none")]
        pub rtt_worst: Option<f64>,
        #[serde(skip_serializing_if="Option::is_none")]
        pub rtt_best: Option<f64>,
        #[serde(skip_serializing_if="Option::is_none")]
        pub rtt_mavg: Option<f64>,
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
            self.last_rx = Some(timestamp);
            if self.last_rx_seq.is_some() {
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

        pub fn rtt_success(&mut self, sent: f64, recv: f64) {
            self.inflight_messages -= 1;
            self.timely_received_messages += 1;
            let rtt = recv - sent;
            
            if let Some(rtt_worst) = self.rtt_worst {
                self.rtt_worst = Some(rtt_worst.max(rtt));
            } else {
                self.rtt_worst = Some(rtt);
            }
            
            if let Some(rtt_best) = self.rtt_best {
                self.rtt_best = Some(rtt_best.min(rtt));
            } else {
                self.rtt_best = Some(rtt);
            }

            if let Some(rtt_mavg) = self.rtt_mavg {
                self.rtt_mavg = Some((rtt_mavg * 9.0 + rtt) / 10.0);
            } else {
                self.rtt_mavg = Some(rtt);
            }
        }
    }

    impl ClientSessionStatistics {
        pub fn from_session_tracker(timestamp: f64, sid: &std::string::String, st: &ClientSessionTracker) -> ClientSessionStatistics {
            return ClientSessionStatistics {
                clocktower_type: "client_session_statistics".to_string(),
                sid: sid.clone(),
                timestamp: timestamp,
                
                seq_unexpected_decrement: st.seq_unexpected_decrement,
                seq_unexpected_increment: st.seq_unexpected_increment,

                sent_messages: st.sent_messages,
                received_messages: st.received_messages,
                timely_received_messages: st.timely_received_messages,

                lost_messages: st.lost_messages,
                inflight_messages: st.inflight_messages,

                rtt_worst: st.rtt_worst,
                rtt_best: st.rtt_best,
                rtt_mavg: st.rtt_mavg,
            }
        }

        pub fn to_json(self) -> Result<std::string::String, serde_json::Error> {
            return serde_json::to_string(&self);
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