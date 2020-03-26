extern crate clap;
extern crate metronome_lib;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
use std::io::Write;
use clap::{Arg, App};
mod hub_lib;
use metronome_lib::datatypes::{MetronomeMessage, WrappedMessage, SessionContainer};
use hub_lib::datatypes::{ServerConfig, WrappedSerializedMessage, ServerSessionStatistics};


const SLEEP_TIME: u64 = 100;
const TIMEOUT_SECONDS: f64 = 5.0;
const HOLE_TIMEOUT_SECONDS: f64 = 1.0;
const STATS_INTERVAL: f64 = 1.0;


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

fn receiver_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, config: ServerConfig, socket: std::net::UdpSocket, receiver_tx: std::sync::mpsc::Sender<WrappedMessage>) {
    let mut rxbuf = [0;65536];
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok((_size, addr)) = socket.recv_from(&mut rxbuf) {
            if let Some(metronome_message) = MetronomeMessage::parse_from_buffer(&rxbuf) {
                if metronome_message.key != config.key {
                    continue;
                }

                let wrapped_message = WrappedMessage {
                    addr: addr,
                    message: metronome_message,
                };

                if let Err(e) = receiver_tx.send(wrapped_message) {
                    eprintln!("failed to send WrappedMessage from receiver thread: {}", e);
                }
            }
        }
    }
}

fn responder_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, socket: std::net::UdpSocket, responder_rx: std::sync::mpsc::Receiver<WrappedSerializedMessage>) {
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(wrapped_message) = responder_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
            loop {
                if let Err(e) = socket.send_to(&wrapped_message.serialized_message, wrapped_message.addr) {
                    eprintln!("failed to sendto() to metronome_client: {}", e);
                } else {
                    break;
                }
            }
        }
    }
}

fn handler_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, handler_receiver_rx: std::sync::mpsc::Receiver<WrappedMessage>, handler_responder_tx: std::sync::mpsc::Sender<WrappedSerializedMessage>, handler_analyzer_tx: std::sync::mpsc::Sender<MetronomeMessage>) {
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(wrapped_message) = handler_receiver_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
            if wrapped_message.message.mode != "ping" {
                continue;
            }

            let original_message = wrapped_message.message.clone();
            let response = wrapped_message.message.get_pong();
            
            match response.as_vec() {
                Ok(serialized) => {
                    if let Err(e) = handler_responder_tx.send(WrappedSerializedMessage {
                        addr: wrapped_message.addr,
                        serialized_message: serialized
                    }) {
                        eprintln!("failed to send WrappedSerializedMessage to sender: {}", e);
                    }
                    if let Err(e) = handler_analyzer_tx.send(original_message) {
                        eprintln!("failed to send MetronomeMessage to analyzer: {}", e);
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

fn analyzer_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, analyzer_rx: std::sync::mpsc::Receiver<MetronomeMessage>, stats_socket: std::net::UdpSocket) {
    let mut session_data: std::collections::HashMap<std::string::String, SessionContainer> = std::collections::HashMap::new();
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(message) = analyzer_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {
            let current_time = metronome_lib::util::get_timestamp();
            if let Some(existing_session_statistics) = session_data.get_mut(&message.sid) {
                let session_statistics: &mut SessionContainer;
                session_statistics = existing_session_statistics;
                session_statistics.seq_analyze(message.seq, current_time);
            } else {
                session_data.insert(message.sid.clone(), SessionContainer::new(message.seq, current_time));
            }
        }
        
        let mut remove_items: Vec<std::string::String> = Vec::new();
        for (session_key, session_container) in session_data.iter_mut() {
            let current_time = metronome_lib::util::get_timestamp();
            let session_deadline = current_time - TIMEOUT_SECONDS;
            let hole_deadline = current_time - HOLE_TIMEOUT_SECONDS;
            let stats_deadline = current_time - STATS_INTERVAL;
            session_container.prune_holes(hole_deadline);

            if session_container.last_rx < session_deadline {
                session_container.last_stats = current_time;
                send_stats(ServerSessionStatistics::from_session_container(session_key, session_container), &stats_socket);
                remove_items.push(session_key.clone());
            } else {
                if session_container.last_stats < stats_deadline {
                    session_container.last_stats = current_time;
                    send_stats(ServerSessionStatistics::from_session_container(session_key, session_container), &stats_socket);
                }
            }
        }
        for remove_item in remove_items.iter() {
            println!("DD");
            session_data.remove(remove_item);
        }

        std::io::stdout().flush().unwrap();
    }
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
                .takes_value(true)
                .required(true)
        )
        .get_matches();
    
    let config = ServerConfig {
        bind: matches.value_of("bind").unwrap().parse().unwrap(),
        key: matches.value_of("key").unwrap().to_string(),
        clocktower: matches.value_of("clocktower").unwrap().parse().unwrap(),
    };

    let socket = prepare_client_socket(config.bind);
    let stats_socket = prepare_stats_socket(config.clocktower);
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
        analyzer_thread(running_analyzer, config_analyzer, analyzer_rx, stats_socket)
    });

    receiver_thd.join().unwrap();
    handler_thd.join().unwrap();
    responder_thd.join().unwrap();
    analyzer_thd.join().unwrap();
}
