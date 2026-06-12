# chattr

A WebRTC video chat app. Peer-to-peer, no server processing media.

## Run

```bash
npm install
npm start
```

Open `http://localhost:3000` in two browser tabs. Enter the same room name to connect.

## HTTPS (for mobile)

```bash
bash setup-certs.sh
npm start
```

Open `https://<your-ip>:3000` on your phone.

## How it works

- Signaling server (Socket.IO) coordinates room joining and SDP/ICE exchange
- Media flows directly between browsers (P2P)
- Chat messages go through the signaling server
- STUN server helps discover public IP for NAT traversal
