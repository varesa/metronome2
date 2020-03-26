extern crate clap;
extern crate metronome_lib;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
use clap::{Arg, App};
use client_lib::datatypes::{ClientConfig, ClientSessionStatistics};
mod client_lib;

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

    if let Err(e) = socket.connect(addr) {
        panic!("failed to connect socket to {}: {}", addr, e);
    }
    
    return socket;
}

fn tx_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ClientConfig, tx_socket: std::net::UdpSocket, tx_stats_tx: std::sync::mpsc::Sender<ClientSessionStatistics>) {

}

fn rx_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ClientConfig, rx_socket: std::net::UdpSocket, rx_stats_tx: std::sync::mpsc::Sender<ClientSessionStatistics>) {

}

fn stats_thread(running: std::sync::Arc<std::sync::atomic::AtomicBool>, _config: ClientConfig, clocktower_socket: std::net::UdpSocket, tx_stats_rx: std::sync::mpsc::Receiver<ClientSessionStatistics>, rx_stats_rx: std::sync::mpsc::Receiver<ClientSessionStatistics>) {

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

    let running_rx = running.clone();
    let running_tx = running.clone();
    let running_stats = running.clone();
    let config_rx = config.clone();
    let config_tx = config.clone();
    let config_stats = config.clone();

    let tx_thd = std::thread::spawn(move || {
        tx_thread(running_tx, config_tx, hub_tx_socket, tx_stats_tx);
    });

    let rx_thd = std::thread::spawn(move || {
        rx_thread(running_rx, config_rx, hub_rx_socket, rx_stats_tx);
    });

    let stats_thd = std::thread::spawn(move || {
        stats_thread(running_stats, config_stats, clocktower_socket, tx_stats_rx, rx_stats_rx);
    });

    tx_thd.join().unwrap();
    rx_thd.join().unwrap();
    stats_thd.join().unwrap();
}
