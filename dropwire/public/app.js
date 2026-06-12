const socket = io();
let pc = null, dc = null, roomId = null;
let receiving = null;

const CHUNK_SIZE = 16384;
const BUFFER_THRESHOLD = 262144;

const $ = (id) => document.getElementById(id);

function join() {
  roomId = $("room-input").value.trim().toLowerCase();
  if (!roomId) return;
  $("lobby").classList.add("hidden");
  $("app").classList.remove("hidden");
  $("room-tag").textContent = roomId;
  socket.emit("join-room", roomId);
}

function leave() {
  cleanup();
  location.reload();
}

function cleanup() {
  if (dc) { dc.close(); dc = null; }
  if (pc) { pc.close(); pc = null; }
  receiving = null;
}

function setStatus(msg, type) {
  const c = $("conn");
  c.className = "conn";
  if (type) c.classList.add(type);
  $("conn-label").textContent = msg;
  $("status").textContent = msg === "Connected" ? "Drop a file to send" : msg;
}

function makePC(initiator) {
  pc = new RTCPeerConnection({
    iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
  });
  pc.onicecandidate = (e) => {
    if (e.candidate)
      socket.emit("ice-candidate", { roomId, candidate: e.candidate });
  };
  pc.oniceconnectionstatechange = () => {
    if (pc.iceConnectionState === "connected" || pc.iceConnectionState === "completed")
      setStatus("Connected", "on");
    else if (pc.iceConnectionState === "disconnected" || pc.iceConnectionState === "failed")
      setStatus("Disconnected", "off");
  };
  if (initiator) {
    dc = pc.createDataChannel("file-transfer");
    setupDC();
  } else {
    pc.ondatachannel = (e) => {
      dc = e.channel;
      setupDC();
    };
  }
  return pc;
}

function setupDC() {
  dc.binaryType = "arraybuffer";
  dc.onopen = () => setStatus("Connected", "on");
  dc.onclose = () => setStatus("Disconnected", "off");
  dc.onmessage = onMessage;
}

socket.on("peer-joined", async () => {
  toast("Peer joined");
  setStatus("Connecting\u2026", "busy");
  pc = makePC(true);
  const offer = await pc.createOffer();
  await pc.setLocalDescription(offer);
  socket.emit("offer", { roomId, sdp: offer.sdp });
});

socket.on("offer", async ({ sdp }) => {
  setStatus("Connecting\u2026", "busy");
  pc = makePC(false);
  await pc.setRemoteDescription(new RTCSessionDescription({ type: "offer", sdp }));
  const answer = await pc.createAnswer();
  await pc.setLocalDescription(answer);
  socket.emit("answer", { roomId, sdp: answer.sdp });
});

socket.on("answer", async ({ sdp }) => {
  await pc.setRemoteDescription(new RTCSessionDescription({ type: "answer", sdp }));
});

socket.on("ice-candidate", async ({ candidate }) => {
  if (pc) {
    try { await pc.addIceCandidate(new RTCIceCandidate(candidate)); } catch (_) {}
  }
});

socket.on("peer-left", () => {
  toast("Peer left", true);
  setStatus("Idle", "off");
  cleanup();
  $("transfer").classList.add("hidden");
  $("received").classList.add("hidden");
});

function onMessage(e) {
  if (typeof e.data === "string") {
    const meta = JSON.parse(e.data);
    if (meta.type === "metadata") {
      receiving = { name: meta.name, size: meta.size, mime: meta.mime, chunks: [], received: 0 };
      $("transfer").classList.remove("hidden");
      $("received").classList.add("hidden");
      setFileMeta("file-icon", "file-name", "file-size", meta.name, meta.size, meta.mime);
      $("file-name").textContent = "Receiving " + meta.name;
      updateProgress(0);
    }
    return;
  }

  if (!receiving) return;
  receiving.chunks.push(e.data);
  receiving.received += e.data.byteLength;
  const pct = Math.round((receiving.received / receiving.size) * 100);
  updateProgress(pct);

  if (receiving.received >= receiving.size) {
    const blob = new Blob(receiving.chunks, { type: receiving.mime });
    const url = URL.createObjectURL(blob);
    $("transfer").classList.add("hidden");
    $("received").classList.remove("hidden");

    const preview = $("preview");
    preview.innerHTML = "";
    if (receiving.mime.startsWith("image/")) {
      const img = document.createElement("img");
      img.src = url;
      preview.appendChild(img);
    }

    const fileName = receiving.name;
    const fileSize = receiving.size;
    const mime = receiving.mime;
    setFileMeta("received-icon", "received-name", "received-size", fileName, fileSize, mime);
    $("received-name").textContent = fileName;

    $("download-btn").onclick = () => {
      const a = document.createElement("a");
      a.href = url;
      a.download = fileName;
      a.style.display = "none";
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    };
    toast("File received!");
    receiving = null;
  }
}

function setFileMeta(iconId, nameId, sizeId, name, size, mime) {
  let icon;
  if (mime.startsWith("image/")) {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>';
  } else if (mime.startsWith("video/")) {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2" ry="2"/></svg>';
  } else if (mime.startsWith("audio/")) {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>';
  } else if (mime.includes("pdf")) {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 15h6"/><path d="M12 12v6"/></svg>';
  } else if (mime.startsWith("text/") || name.includes(".zip") || name.includes(".rar")) {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>';
  } else {
    icon = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>';
  }
  $(iconId).innerHTML = icon;
  $(nameId).textContent = name;
  $(sizeId).textContent = formatSize(size);
}

function formatSize(bytes) {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1048576).toFixed(1) + " MB";
}

function updateProgress(pct) {
  $("progress-bar").style.width = pct + "%";
  $("progress-pct").textContent = pct + "%";
}

let sendStart = 0;

function sendFile(file) {
  if (!dc || dc.readyState !== "open") {
    toast("No peer connected", true);
    return;
  }

  const reader = new FileReader();
  reader.onload = () => {
    const data = reader.result;
    dc.send(JSON.stringify({ type: "metadata", name: file.name, size: file.size, mime: file.type }));

    $("transfer").classList.remove("hidden");
    $("received").classList.add("hidden");
    setFileMeta("file-icon", "file-name", "file-size", file.name, file.size, file.type);
    $("file-name").textContent = "Sending " + file.name;
    updateProgress(0);

    const total = data.byteLength;
    let offset = 0;
    sendStart = Date.now();

    function sendNext() {
      while (offset < total && dc.bufferedAmount < BUFFER_THRESHOLD) {
        const end = Math.min(offset + CHUNK_SIZE, total);
        dc.send(data.slice(offset, end));
        offset = end;
        const pct = Math.round((offset / total) * 100);
        updateProgress(pct);
        const elapsed = (Date.now() - sendStart) / 1000;
        if (elapsed > 0) {
          const speed = (offset / elapsed) / 1024;
          $("progress-speed").textContent = speed > 1024
            ? (speed / 1024).toFixed(1) + " MB/s"
            : speed.toFixed(0) + " KB/s";
        }
      }

      if (offset < total) {
        setTimeout(sendNext, 30);
      } else {
        $("progress-speed").textContent = "";
        toast("File sent!");
      }
    }

    sendNext();
  };
  reader.readAsArrayBuffer(file);
}

const dz = $("dropzone");

dz.addEventListener("dragover", (e) => {
  e.preventDefault();
  dz.classList.add("dragover");
});

dz.addEventListener("dragleave", () => {
  dz.classList.remove("dragover");
});

dz.addEventListener("drop", (e) => {
  e.preventDefault();
  dz.classList.remove("dragover");
  const file = e.dataTransfer.files[0];
  if (file) sendFile(file);
});

$("file-input").addEventListener("change", () => {
  const file = $("file-input").files[0];
  if (file) sendFile(file);
  $("file-input").value = "";
});

$("room-input").addEventListener("keypress", (e) => {
  if (e.key === "Enter") join();
});

$("reset-btn").addEventListener("click", () => {
  $("received").classList.add("hidden");
  $("preview").innerHTML = "";
});

function toast(msg, err) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = (err ? "! " : "") + msg;
  $("toast-wrap").appendChild(el);
  setTimeout(() => el.remove(), 2800);
}
