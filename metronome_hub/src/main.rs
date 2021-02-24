extern crate clap;
extern crate metronome_lib;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
use clap::{Arg, App};
mod hub_lib;
use metronome_lib::datatypes::{MetronomeMessage, MessageWithSize, OriginInfoMessage, SessionContainer};
use hub_lib::datatypes::{ServerConfig, WrappedSerializedMessage, ServerSessionStatistics};


const SLEEP_TIME: u64 = 100;
const TIMEOUT_SECONDS: f64 = (7*24*60*60) as f64;
const HOLE_TIMEOUT_SECONDS: f64 = 1.0;


fn prepare_client_socket(addr: std::net::SocketAddr) -> std::net::UdpSocket {
    let socket: std::net::UdpSocket;

    match std::net::UdpSocket::bind(addr) {
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

    return socket;
}

fn prepare_stats_socket(addr: std::net::SocketAddr) -> std::net::UdpSocket {
    let socket: std::net::UdpSocket;

    match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(bound_socket) => {
            socket = bound_socket;
        },
        Err(_) => {
            panic!("failed to bind socket");
        }
    }

    if let Err(e) = socket.connect(addr) {
        panic!("failed to connect clocktower socket to {}: {}", addr, e);
    }
    
    return socket;
}

fn receiver_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, config: ServerConfig, socket: std::net::UdpSocket, receiver_tx: std::sync::mpsc::Sender<OriginInfoMessage>) {
    let mut rxbuf = [0;65536];
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok((size, addr)) = socket.recv_from(&mut rxbuf) {
            if let Some(metronome_message) = MetronomeMessage::parse_from_buffer(&rxbuf) {
                if metronome_message.key != config.key {
                    continue;
                }

                let origin_info_message = OriginInfoMessage {
                    timestamp: metronome_lib::util::get_timestamp(),
                    addr: addr,
                    message_with_size: MessageWithSize {
                        message_raw_size: size,
                        message: metronome_message,
                    },
                };

                if let Err(e) = receiver_tx.send(origin_info_message) {
                    eprintln!("failed to send OriginInfoMessage from receiver thread: {}", e);
                }
            }
        }
    }
}

fn responder_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, socket: std::net::UdpSocket, responder_rx: std::sync::mpsc::Receiver<WrappedSerializedMessage>) {
    let mut last_send_error_printed: f64 = 0.0;
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(wrapped_message) = responder_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
            loop {
                if let Err(e) = socket.send_to(&wrapped_message.serialized_message, wrapped_message.addr) {
                    let current_time = metronome_lib::util::get_timestamp();
                    if (current_time - last_send_error_printed) > 10.0 { 
                        eprintln!("failed to sendto() to metronome_client {}: {}", wrapped_message.addr, e);
                        last_send_error_printed = current_time;
                    }
                } else {
                    break;
                }
            }
        }
    }
}

fn handler_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, handler_receiver_rx: std::sync::mpsc::Receiver<OriginInfoMessage>, handler_responder_tx: std::sync::mpsc::Sender<WrappedSerializedMessage>, handler_analyzer_tx: std::sync::mpsc::Sender<OriginInfoMessage>) {
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(origin_info_message) = handler_receiver_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
            if origin_info_message.message_with_size.message.mode != "ping" {
                continue;
            }

            let response = origin_info_message.message_with_size.clone().message.get_pong();
            
            match response.as_vec() {
                Ok(serialized) => {
                    if let Err(e) = handler_responder_tx.send(WrappedSerializedMessage {
                        addr: origin_info_message.addr,
                        serialized_message: serialized
                    }) {
                        eprintln!("failed to send WrappedSerializedMessage to sender: {}", e);
                    }
                    if let Err(e) = handler_analyzer_tx.send(origin_info_message) {
                        eprintln!("failed to send MessageWithSize to analyzer: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("failed to serialize MetronomeMessage for transmission: {}", e);
                }
            }
        }
    }
}

fn send_stats(stats: ServerSessionStatistics, stats_socket: &std::net::UdpSocket) {
    if let Ok(stats_json) = stats.to_json() {
        let message_bytes = stats_json.into_bytes();
        if let Err(e) = stats_socket.send(&message_bytes) {
            eprintln!("failed to send statistics to clocktower: {}", e);
        }
    }
}

fn analyzer_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, config: ServerConfig, analyzer_rx: std::sync::mpsc::Receiver<OriginInfoMessage>, clocktowers: Vec<std::net::UdpSocket>) {
    let session_data_arced: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<std::string::String, SessionContainer>>> = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let session_data_arced_inserter = session_data_arced.clone();
    let running_inserter = running.clone();
    let session_data_inserter_thread = std::thread::spawn(move || {
        while running_inserter.load(std::sync::atomic::Ordering::Relaxed) {
            if let Ok(origin_info_message) = analyzer_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
                let message_with_size = origin_info_message.message_with_size;
                if let Ok(mut session_data) = session_data_arced_inserter.lock() {
                    if let Some(existing_session_statistics) = session_data.get_mut(&message_with_size.message.sid) {
                        let session_statistics: &mut SessionContainer;
                        session_statistics = existing_session_statistics;
                        session_statistics.seq_analyze(message_with_size.message.seq, message_with_size.message_raw_size, origin_info_message.timestamp);
                    } else {
                        session_data.insert(message_with_size.message.sid.clone(), SessionContainer::new(message_with_size.message.seq, message_with_size.message_raw_size, origin_info_message.timestamp));
                    }
                }
            }
        }
    });
    
    let session_data_arced_scanner = session_data_arced.clone();
    let running_scanner = running.clone();
    let session_scanner_thread = std::thread::spawn(move || {
        let mut last_session_data_scan: f64 = 0.0;
        let session_data_scan_interval: f64 = TIMEOUT_SECONDS.min(config.stats_interval).min(HOLE_TIMEOUT_SECONDS);

        while running_scanner.load(std::sync::atomic::Ordering::Relaxed) {
            let current_time = metronome_lib::util::get_timestamp();
            if last_session_data_scan < (current_time - session_data_scan_interval) {
                last_session_data_scan = current_time;
                let mut remove_items: Vec<std::string::String> = Vec::new();
                if let Ok(mut session_data) = session_data_arced_scanner.lock() {
                    for (session_key, session_container) in session_data.iter_mut() {
                        let session_deadline = current_time - TIMEOUT_SECONDS;
                        let hole_deadline = current_time - HOLE_TIMEOUT_SECONDS;
                        let stats_deadline = current_time - config.stats_interval;
                        session_container.prune_holes(hole_deadline);

                        if session_container.last_rx < session_deadline {
                            session_container.last_stats = current_time;
                            for clocktower in clocktowers.iter() {
                                send_stats(ServerSessionStatistics::from_session_container(session_key, session_container), &clocktower);
                            }
                            remove_items.push(session_key.clone());
                        } else {
                            if session_container.last_stats < stats_deadline {
                                session_container.last_stats = current_time;
                                for clocktower in clocktowers.iter() {
                                    send_stats(ServerSessionStatistics::from_session_container(session_key, session_container), &clocktower);
                                }
                            }
                        }
                    }
                    for remove_item in remove_items.iter() {
                        session_data.remove(remove_item);
                    }
                }
            }
            let sleeptime = std::time::Duration::from_millis(100);
            std::thread::sleep(sleeptime);
        }
    });

    session_data_inserter_thread.join().unwrap();
    session_scanner_thread.join().unwrap();
}

fn main() {
    let matches = App::new("metronome-server")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("bind")
                .short("b")
                .long("bind")
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
            Arg::with_name("clocktower")
                .short("c")
                .long("clocktower")
                .multiple(true)
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("stats_interval")
                .long("stats-interval")
                .takes_value(true)
                .default_value("1.0")
        )
        .get_matches();
    
    let mut clocktowers: Vec<std::net::UdpSocket> = Vec::new();
    if let Some(clocktower_strings) = matches.values_of("clocktower") {
        for clocktower_string in clocktower_strings {
            let clocktower_address: std::net::SocketAddr = clocktower_string.parse().unwrap();
            let clocktower_socket = prepare_stats_socket(clocktower_address);
            clocktowers.push(clocktower_socket);
        }
    }

    let config = ServerConfig {
        bind: matches.value_of("bind").unwrap().parse().unwrap(),
        key: matches.value_of("key").unwrap().to_string(),
        stats_interval: matches.value_of("stats_interval").unwrap().parse().unwrap(),
    };

    let socket = prepare_client_socket(config.bind);
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    let (receiver_tx, handler_receiver_rx) = std::sync::mpsc::channel();
    let (handler_responder_tx, responder_rx) = std::sync::mpsc::channel();
    let (handler_analyzer_tx, analyzer_rx) = std::sync::mpsc::channel();

    let socket_receiver = socket.try_clone().unwrap();
    let socket_responder = socket.try_clone().unwrap();
    let running_receiver = running.clone();
    let running_handler = running.clone();
    let running_responder = running.clone();
    let running_analyzer = running.clone();
    let config_receiver = config.clone();
    let config_handler = config.clone();
    let config_responder = config.clone();
    let config_analyzer = config.clone();

    let receiver_thd = std::thread::spawn(move || {
        receiver_thread(running_receiver, config_receiver, socket_receiver, receiver_tx)
    });

    let handler_thd = std::thread::spawn(move || {
        handler_thread(running_handler, config_handler, handler_receiver_rx, handler_responder_tx, handler_analyzer_tx)
    });

    let responder_thd = std::thread::spawn(move || {
        responder_thread(running_responder, config_responder, socket_responder, responder_rx)
    });

    let analyzer_thd = std::thread::spawn(move || {
        analyzer_thread(running_analyzer, config_analyzer, analyzer_rx, clocktowers)
    });

    receiver_thd.join().unwrap();
    handler_thd.join().unwrap();
    responder_thd.join().unwrap();
    analyzer_thd.join().unwrap();
}
