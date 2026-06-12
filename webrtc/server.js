import express from "express";
import { createServer } from "http";
import { createServer as createHttpsServer } from "https";
import { Server } from "socket.io";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import os from "os";
import fs from "fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const app = express();
const PORT = process.env.PORT || 3000;
const HOST = "0.0.0.0";

const certPath = join(__dirname, "certs", "cert.pem");
const keyPath = join(__dirname, "certs", "key.pem");
const hasCerts = fs.existsSync(certPath) && fs.existsSync(keyPath);

function getLocalIP() {
  const interfaces = os.networkInterfaces();
  for (const name of Object.keys(interfaces)) {
    for (const iface of interfaces[name]) {
      if (iface.family === "IPv4" && !iface.internal) return iface.address;
    }
  }
  return "127.0.0.1";
}
const localIP = getLocalIP();

app.use(express.static(join(__dirname, "public")));

const httpServer = createServer(app);

if (hasCerts) {
  const options = {
    cert: fs.readFileSync(certPath),
    key: fs.readFileSync(keyPath),
  };
  const httpsServer = createHttpsServer(options, app);
  const io = new Server(httpsServer, {
    cors: { origin: "*", methods: ["GET", "POST"] }
  });
  setupSocket(io);

  httpsServer.listen(PORT, HOST, () => {
    console.log("\n  🚀 WebRTC Video Chat Server (HTTPS)\n");
    console.log(`  Local:    https://localhost:${PORT}`);
    console.log(`  Network:  https://${localIP}:${PORT}`);
    console.log("\n  ⚠  Mobile browsers need HTTPS for camera access.");
    console.log("  📱 Open the Network URL on your phone.");
    console.log("  🔒 Accept the 'Not Secure' warning (self-signed cert).\n");
  });

  httpServer.listen(PORT + 1, HOST, () => {
    console.log(`  🔀 HTTP on :${PORT + 1} → redirects to HTTPS :${PORT}\n`);
  });
} else {
  const io = new Server(httpServer, {
    cors: { origin: "*", methods: ["GET", "POST"] }
  });
  setupSocket(io);

  httpServer.listen(PORT, HOST, () => {
    console.log("\n  🚀 WebRTC Video Chat Server (HTTP — no camera on mobile)\n");
    console.log(`  Local:    http://localhost:${PORT}`);
    console.log(`  Network:  http://${localIP}:${PORT}`);
    console.log("\n  ⚠  Mobile browsers need HTTPS for camera access.");
    console.log(`  Run: bash setup-certs.sh && npm start\n`);
  });
}

const users = new Map();

function setupSocket(io) {
  io.on("connection", (socket) => {
    console.log("User connected:", socket.id);

    socket.on("join-room", (roomId) => {
      socket.join(roomId);
      users.set(socket.id, { roomId });

      const clients = io.sockets.adapter.rooms.get(roomId);
      const numClients = clients ? clients.size : 0;

      if (numClients === 2) {
        socket.to(roomId).emit("peer-joined");
      }

      console.log(`${socket.id} joined room ${roomId} (${numClients} users)`);
    });

    socket.on("offer", ({ roomId, sdp }) => {
      socket.to(roomId).emit("offer", { sdp });
    });

    socket.on("answer", ({ roomId, sdp }) => {
      socket.to(roomId).emit("answer", { sdp });
    });

    socket.on("ice-candidate", ({ roomId, candidate }) => {
      socket.to(roomId).emit("ice-candidate", { candidate });
    });

    socket.on("chat-message", ({ roomId, message }) => {
      socket.to(roomId).emit("chat-message", { message, from: socket.id });
    });

    socket.on("disconnect", () => {
      const user = users.get(socket.id);
      if (user) {
        socket.to(user.roomId).emit("peer-left");
        users.delete(socket.id);
      }
      console.log("User disconnected:", socket.id);
    });
  });
}
