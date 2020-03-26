import time
import socket
import json
import threading
import argparse
from prometheus_client import start_http_server
from prometheus_client.core import GaugeMetricFamily, CounterMetricFamily, REGISTRY


parser = argparse.ArgumentParser(description='metronome clocktower')
parser.add_argument('-b', '--bind-address', required=False, help='bind address for clocktower messages', default='0.0.0.0')
parser.add_argument('-p', '--bind-port', required=False, help='bind port for clocktower messages', default='4444', type=int)
parser.add_argument('-e', '--exporter-port', required=False, help='bind port for prometheus exporter', default='8415', type=int)
args = parser.parse_args()


SESSION_TIMEOUT = 10.0

sessions = {}
sessions_lock = threading.Lock()
msglistener = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
msglistener.bind((args.bind_address, args.bind_port))


class CustomCollector(object):
    def collect(self):
        rxmsg = CounterMetricFamily(
            'metronome_server_received_messages',
            'Messages received by the metronome server',
            labels=['sid']
        )
        holes_created = CounterMetricFamily(
            'metronome_server_holes_created',
            'Holes created within session',
            labels=['sid']
        )
        holes_closed = CounterMetricFamily(
            'metronome_server_holes_closed',
            'Holes closed within session',
            labels=['sid']
        )
        holes_timed_out = CounterMetricFamily(
            'metronome_server_holes_timed_out',
            'Holes timed out within session',
            labels=['sid']
        )
        holes_current = GaugeMetricFamily(
            'metronome_server_holes_current',
            'Current holes within session',
            labels=['sid']
        )

        for sid, session_info in sessions.items():
            rxmsg.add_metric([sid], session_info.get('received_messages'), timestamp=session_info.get('timestamp'))
            holes_created.add_metric([sid], session_info.get('holes_created'), timestamp=session_info.get('timestamp'))
            holes_closed.add_metric([sid], session_info.get('holes_closed'), timestamp=session_info.get('timestamp'))
            holes_timed_out.add_metric([sid], session_info.get('holes_timed_out'), timestamp=session_info.get('timestamp'))
            holes_current.add_metric([sid], session_info.get('holes_current'), timestamp=session_info.get('timestamp'))

        yield rxmsg
        yield holes_created
        yield holes_closed
        yield holes_timed_out
        yield holes_current


def inject_server_session_statistics(payload):
    global sessions
    global sessions_lock

    sid = payload.get('sid')
    with sessions_lock:
        print(payload)
        sessions[sid] = payload


def cleanup_sessions():
    global sessions
    global sessions_lock

    while True:
        current_time = time.time()
        remove_list = []
        with sessions_lock:
            for sid, session in sessions.items():
                age = current_time - session.get('timestamp')
                if age > SESSION_TIMEOUT:
                    remove_list.append(sid)
            for sid in remove_list:
                del sessions[sid]
        time.sleep(1)


def main():
    threading.Thread(target=cleanup_sessions, daemon=True).start()
    REGISTRY.register(CustomCollector())
    start_http_server(args.exporter_port)
    while True:
        try:
            payload = json.loads(msglistener.recv(4096).decode('ascii', errors='ignore'))
            if payload.get('clocktower_type') == 'server_session_statistics':
                inject_server_session_statistics(payload)
        except json.decoder.JSONDecodeError:
            pass


if __name__ == '__main__':
    main()

