import socket
import json


msglistener = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
msglistener.bind(('0.0.0.0', 4444))
while True:
    print(json.loads(msglistener.recv(4096).decode('ascii')))
