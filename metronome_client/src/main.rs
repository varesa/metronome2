extern crate clap;
extern crate metronome_lib;
extern crate serde;
extern crate serde_json;
extern crate single_value_channel;
#[macro_use] extern crate serde_derive;
use clap::{Arg, App};
use client_lib::datatypes::{ClientConfig, ClientSessionTracker, RTTMeasurement};
use metronome_lib::datatypes::{MetronomeMessage, TimestampedMessage, SessionContainer, MessageWithSize};
mod client_lib;

const SLEEP_TIME: u64 = 100;

fn prepare_connect_socket(addr: std::net::SocketAddr) -> std::net::UdpSocket {
    let socket: std::net::UdpSocket;

    match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(bound_socket) => {
            socket = bound_socket;
        },
        Err(_) => {
            panic!("failed to bind socket");
        }
    }

    if let Err(_) = socket.set_read_timeout(Some(std::time::Duration::from_millis(SLEEP_TIME))) {
        panic!("failed to set socket read timeout!");
    }

    if let Err(e) = socket.connect(addr) {
        panic!("failed to connect socket to {}: {}", addr, e);
    }
    
    return socket;
}

fn tx_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, config: ClientConfig, tx_socket: std::net::UdpSocket, tx_stats_tx: std::sync::mpsc::Sender<RTTMeasurement>, mut target_pps: single_value_channel::Receiver<u64>) {
    let mut msg_seq: u64 = 0;
    let payload = std::iter::repeat("X").take(config.payload_size).collect::<String>();
    let mut next_tx_at = 0.0;
    let mut msg: MetronomeMessage = MetronomeMessage {
        mode: "ping".to_string(),
        payload: Some(payload),
        mul: config.balance,
        seq: msg_seq,
        key: config.key,
        sid: config.sid,
    };
    let mut pps_sleeptime: f64;
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        pps_sleeptime = 1.0/(*target_pps.latest() as f64);

        let current_time = metronome_lib::util::get_timestamp();
        if current_time >= next_tx_at {
            msg.seq = msg_seq;
            match msg.as_vec() {
                Ok(serialized) => {
                    if let Err(e) = tx_socket.send(&serialized) {
                        eprintln!("failed to send message to hub: {}", e);
                    } else {
                        let rttmeas = RTTMeasurement {
                            seq: msg_seq,
                            timestamp: current_time,
                        };
                        if let Err(e) = tx_stats_tx.send(rttmeas) {
                            eprintln!("failed to send RTT measurement to stats thread: {}", e);
                        }
                        msg_seq += 1;
                    }
                },
                Err(e) => {
                    eprintln!("failed to serialize MetronomeMessage for transmission: {}", e);
                }
            }
        }
        next_tx_at = current_time + pps_sleeptime;
        if config.use_sleep {
            // Fixme
            let sleeptime = std::time::Duration::from_micros(100);
            std::thread::sleep(sleeptime);
        }
    }
}

fn rx_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, config: ClientConfig, rx_socket: std::net::UdpSocket, rx_stats_tx: std::sync::mpsc::Sender<TimestampedMessage>) {
    let mut rxbuf = [0;65536];
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(size) = rx_socket.recv(&mut rxbuf) {
            let timestamp = metronome_lib::util::get_timestamp();
            if let Some(metronome_message) = MetronomeMessage::parse_from_buffer(&rxbuf) {
                if metronome_message.key != config.key {
                    continue;
                }
                if metronome_message.sid != config.sid {
                    continue;
                }
                let timestamped_message = TimestampedMessage {
                    timestamp: timestamp,
                    message_with_size: MessageWithSize {
                        message_raw_size: size,
                        message: metronome_message,
                    },
                };
                if let Err(e) = rx_stats_tx.send(timestamped_message) {
                    eprintln!("failed to send MetronomeMessage to stats thread from rx thread: {}", e);
                }
            }
        }
    }
}

fn stats_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ClientConfig, clocktower_socket: std::net::UdpSocket, tx_stats_rx: std::sync::mpsc::Receiver<RTTMeasurement>, rx_stats_rx: std::sync::mpsc::Receiver<TimestampedMessage>, pps_updater: single_value_channel::Updater<u64>) {
    let mut tracker: std::collections::HashMap<u64, RTTMeasurement> = std::collections::HashMap::new();
    let mut stats: ClientSessionTracker = ClientSessionTracker::new();
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(rtt_measurement) = tx_stats_rx.try_recv() {
            stats.outgoing(rtt_measurement.timestamp);
            tracker.insert(rtt_measurement.seq, rtt_measurement);
        }
        if let Ok(timestamped_message) = rx_stats_rx.try_recv() {
            let message = &timestamped_message.message_with_size.message;
            stats.incoming(timestamped_message.timestamp, message.seq);
        }
    }
}

fn main() {
    let matches = App::new("metronome-client")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("pps-max")
                .short("p")
                .long("pps-max")
                .takes_value(true)
                .default_value("1")
        )
        .arg(
            Arg::with_name("use-sleep")
                .short("S")
                .long("use-sleep")
        )
        .arg(
            Arg::with_name("payload-size")
                .short("s")
                .long("payload-size")
                .takes_value(true)
                .default_value("1")
        )
        .arg(
            Arg::with_name("balance")
                .short("b")
                .long("balance")
                .takes_value(true)
                .default_value("1")
        )
        .arg(
            Arg::with_name("remote")
                .short("r")
                .long("remote")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("clocktower")
                .short("c")
                .long("clocktower")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("session_id")
                .short("i")
                .long("session-id")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("probe_id")
                .short("P")
                .long("probe-id")
                .takes_value(true)
                .default_value("")
        )
        .get_matches();

    let config = ClientConfig {
        pps_limit: matches.value_of("pps-max").unwrap().parse().unwrap(),
        payload_size: matches.value_of("payload-size").unwrap().parse().unwrap(),
        use_sleep: matches.is_present("use-sleep"),
        balance: matches.value_of("balance").unwrap().parse().unwrap(),
        remote: matches.value_of("remote").unwrap().parse().unwrap(),
        clocktower: matches.value_of("clocktower").unwrap().parse().unwrap(),
        key: matches.value_of("key").unwrap().to_string(),
        sid: matches.value_of("session_id").unwrap().to_string(),
    };

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    
    let hub_socket = prepare_connect_socket(config.remote);
    let clocktower_socket = prepare_connect_socket(config.clocktower);

    let hub_rx_socket = hub_socket.try_clone().unwrap();
    let hub_tx_socket = hub_socket.try_clone().unwrap();

    let (rx_stats_tx, rx_stats_rx) = std::sync::mpsc::channel();
    let (tx_stats_tx, tx_stats_rx) = std::sync::mpsc::channel();
    let (pps_receiver, pps_updater) = single_value_channel::channel_starting_with(config.pps_limit);

    let running_rx = running.clone();
    let running_tx = running.clone();
    let running_stats = running.clone();
    let config_rx = config.clone();
    let config_tx = config.clone();
    let config_stats = config.clone();

    let tx_thd = std::thread::spawn(move || {
        tx_thread(running_tx, config_tx, hub_tx_socket, tx_stats_tx, pps_receiver);
    });

    let rx_thd = std::thread::spawn(move || {
        rx_thread(running_rx, config_rx, hub_rx_socket, rx_stats_tx);
    });

    let stats_thd = std::thread::spawn(move || {
        stats_thread(running_stats, config_stats, clocktower_socket, tx_stats_rx, rx_stats_rx, pps_updater);
    });

    tx_thd.join().unwrap();
    rx_thd.join().unwrap();
    stats_thd.join().unwrap();
}
