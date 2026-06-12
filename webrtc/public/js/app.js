const socket = io();
let pc = null,
  localStream = null,
  roomId = null,
  screenShare = null;
let audioOn = true,
  videoOn = true,
  chatOpen = false;

const STUN = { iceServers: [{ urls: "stun:stun.l.google.com:19302" }] };

const $ = (id) => document.getElementById(id);

async function joinRoom() {
  roomId = $("room-input").value.trim();
  if (!roomId) return;
  $("lobby").classList.add("hidden");
  $("app").classList.remove("hidden");
  $("room-tag").textContent = "#" + roomId;
  await startLocal();
  socket.emit("join-room", roomId);
}

function leaveRoom() {
  if (pc) {
    pc.close();
    pc = null;
  }
  if (localStream) localStream.getTracks().forEach((t) => t.stop());
  if (screenShare) screenShare.getTracks().forEach((t) => t.stop());
  location.reload();
}

async function startLocal() {
  try {
    localStream = await navigator.mediaDevices.getUserMedia({
      video: true,
      audio: true,
    });
    $("local-video").srcObject = localStream;
    $("local-ph").classList.add("hidden");
    $("local-av").classList.add("on");
    toast("Camera ready");
  } catch (e) {
    toast("Camera access denied", true);
  }
}

function makePC() {
  pc = new RTCPeerConnection(STUN);
  if (localStream)
    localStream.getTracks().forEach((t) => pc.addTrack(t, localStream));
  pc.ontrack = (e) => {
    $("remote-video").srcObject = e.streams[0];
    $("remote-ph").classList.add("hidden");
  };
  pc.onicecandidate = (e) => {
    if (e.candidate)
      socket.emit("ice-candidate", { roomId, candidate: e.candidate });
  };
  pc.oniceconnectionstatechange = () => status(pc.iceConnectionState);
  return pc;
}

function status(s) {
  const dot = $("conn-dot"),
    label = $("conn-label");
  dot.className = "dot";
  if (s === "connected" || s === "completed") {
    dot.classList.add("on");
    label.textContent = "Connected";
  } else if (s === "disconnected" || s === "failed") {
    dot.classList.add("off");
    label.textContent = "Disconnected";
  } else {
    dot.classList.add("busy");
    label.textContent = "Connecting";
  }
}

socket.on("peer-joined", async () => {
  toast("Someone joined");
  $("remote-label").textContent = "Peer";
  $("remote-av").textContent = "P";
  $("remote-av").classList.add("on");
  pc = makePC();
  const offer = await pc.createOffer();
  await pc.setLocalDescription(offer);
  socket.emit("offer", { roomId, sdp: offer.sdp });
});

socket.on("offer", async ({ sdp }) => {
  toast("Incoming...");
  $("remote-label").textContent = "Connecting";
  $("remote-av").textContent = "P";
  $("remote-av").classList.add("on");
  pc = makePC();
  await pc.setRemoteDescription(
    new RTCSessionDescription({ type: "offer", sdp }),
  );
  const answer = await pc.createAnswer();
  await pc.setLocalDescription(answer);
  socket.emit("answer", { roomId, sdp: answer.sdp });
});

socket.on("answer", async ({ sdp }) => {
  await pc.setRemoteDescription(
    new RTCSessionDescription({ type: "answer", sdp }),
  );
  toast("Connected!");
});

socket.on("ice-candidate", async ({ candidate }) => {
  if (pc) {
    try {
      await pc.addIceCandidate(new RTCIceCandidate(candidate));
    } catch (_) {}
  }
});

socket.on("peer-left", () => {
  toast("Peer left", true);
  $("remote-video").srcObject = null;
  $("remote-ph").classList.remove("hidden");
  $("remote-label").textContent = "Peer left";
  $("remote-av").textContent = "?";
  $("remote-av").classList.remove("on");
  if (pc) {
    pc.close();
    pc = null;
  }
  status("disconnected");
});

function toggleAudio() {
  if (!localStream) return;
  audioOn = !audioOn;
  localStream.getAudioTracks().forEach((t) => (t.enabled = audioOn));
  $("c-mic").classList.toggle("off", !audioOn);
}

function toggleVideo() {
  if (!localStream) return;
  videoOn = !videoOn;
  localStream.getVideoTracks().forEach((t) => (t.enabled = videoOn));
  $("c-cam").classList.toggle("off", !videoOn);
  $("local-ph").classList.toggle("hidden", videoOn);
}

async function toggleScreenShare() {
  if (screenShare) {
    screenShare.getTracks().forEach((t) => t.stop());
    screenShare = null;
    if (pc) {
      const s = pc.getSenders().find((s) => s.track?.kind === "video");
      if (s && localStream) s.replaceTrack(localStream.getVideoTracks()[0]);
    }
    $("c-screen").classList.remove("off");
    return;
  }
  try {
    screenShare = await navigator.mediaDevices.getDisplayMedia({ video: true });
    if (pc) {
      const s = pc.getSenders().find((s) => s.track?.kind === "video");
      if (s) s.replaceTrack(screenShare.getVideoTracks()[0]);
    }
    $("c-screen").classList.add("off");
    screenShare.getVideoTracks()[0].onended = toggleScreenShare;
  } catch (_) {}
}

function toggleChat() {
  chatOpen = !chatOpen;
  $("sidebar").classList.toggle("open", chatOpen);
  if (chatOpen) setTimeout(() => $("chat-inp").focus(), 100);
}

document.addEventListener("keydown", (e) => {
  if (e.key === "c" && !e.ctrlKey && !e.metaKey && !e.target.closest("input"))
    toggleChat();
});

socket.on("chat-message", ({ message }) => addMsg(message, "them"));

function sendMsg() {
  const inp = $("chat-inp"),
    txt = inp.value.trim();
  if (!txt) return;
  socket.emit("chat-message", { roomId, message: txt });
  addMsg(txt, "you");
  inp.value = "";
}

function addMsg(text, who) {
  const el = $("chat-msgs");
  const empty = el.querySelector(".empty");
  if (empty) el.innerHTML = "";
  const t = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  const d = document.createElement("div");
  d.className = "bubble " + who;
  d.innerHTML =
    '<div class="inner">' +
    esc(text) +
    '</div><div class="meta">' +
    t +
    "</div>";
  el.appendChild(d);
  el.scrollTop = el.scrollHeight;
}

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function toast(msg, err) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = (err ? "! " : "") + msg;
  $("toast-wrap").appendChild(el);
  setTimeout(() => el.remove(), 2800);
}

$("room-input").addEventListener("keypress", (e) => {
  if (e.key === "Enter") joinRoom();
});
