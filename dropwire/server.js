const express = require("express");
const http = require("http");
const os = require("os");
const { Server } = require("socket.io");

const app = express();
app.use(express.static("public"));

const server = http.createServer(app);
const io = new Server(server);

const PORT = process.env.PORT || 4000;

function getLocalIP() {
  const ifaces = os.networkInterfaces();
  for (const name of Object.keys(ifaces)) {
    for (const iface of ifaces[name]) {
      if (iface.family === "IPv4" && !iface.internal) return iface.address;
    }
  }
  return "127.0.0.1";
}

io.on("connection", (socket) => {
  socket.on("join-room", (roomId) => {
    socket.join(roomId);
    const peers = io.sockets.adapter.rooms.get(roomId);
    if (peers && peers.size > 1) {
      socket.to(roomId).emit("peer-joined");
    }
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

  socket.on("disconnecting", () => {
    for (const room of socket.rooms) {
      if (room !== socket.id) {
        socket.to(room).emit("peer-left");
      }
    }
  });
});

server.listen(PORT, () => {
  const ip = getLocalIP();
  console.log("local:  http://localhost:" + PORT);
  console.log("network: http://" + ip + ":" + PORT);
});
