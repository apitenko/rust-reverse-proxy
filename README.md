quick & dirty UDP tunnel

___

Host mode: `./rust-reverse-proxy.exe proxy -c 3001 -s 3002`

`-c CLIENT_PORT` (outside clients connect here, UDP)

`-s SERVER_PORT` (another copy of this program running in the client mode should connect here, QUIC)

Both run on localhost ipv4

___

Client mode: `./rust-reverse-proxy.exe connect -r 3000 -a 127.0.0.1 -p 3002`

`-r RECEIVER_PORT` (UDP port of your local program that will receive the traffic)

`-a REMOTE_ADDRESS` (ip address of this program running in the host mode)

`-p REMOTE_PORT` (port of this program running in the host mode)