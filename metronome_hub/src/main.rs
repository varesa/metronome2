extern crate clap;
extern crate metronome_lib;
use clap::{Arg, App};
mod hub_lib;
use metronome_lib::datatypes::{MetronomeMessage, WrappedMessage};
use hub_lib::datatypes::{ServerConfig, HubStatistics, WrappedSerializedMessage};


const SLEEP_TIME: u64 = 100;


fn prepare_socket(addr: std::net::SocketAddr) -> std::net::UdpSocket {
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

fn analyzer_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ServerConfig, analyzer_rx: std::sync::mpsc::Receiver<MetronomeMessage>) {
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(_message) = analyzer_rx.recv_timeout(std::time::Duration::from_millis(SLEEP_TIME)) {

        }
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
                .default_value("0.0.0.0:9150")
        )
        .get_matches();
    
    let config = ServerConfig {
        bind: matches.value_of("bind").unwrap().parse().unwrap(),
        key: matches.value_of("key").unwrap().to_string(),
        clocktower: matches.value_of("clocktower").unwrap().parse().unwrap(),
    };

    let socket = prepare_socket(config.bind);
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
        analyzer_thread(running_analyzer, config_analyzer, analyzer_rx)
    });

    receiver_thd.join().unwrap();
    handler_thd.join().unwrap();
    responder_thd.join().unwrap();
    analyzer_thd.join().unwrap();
}
