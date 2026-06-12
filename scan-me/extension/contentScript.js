(() => {
  const api = typeof chrome !== 'undefined' ? chrome : browser;

  if (window.__qr_selector_active) return;
  window.__qr_selector_active = true;

  const styleHref = api.runtime.getURL('selector.css');

  const overlay = document.createElement('div');
  overlay.className = 'qr-overlay';
  overlay.innerHTML = `
    <div class="qr-backdrop"></div>
    <div class="qr-hud">
      <div class="qr-hud-top">
        <div>
          <div class="qr-brand">QR Reader</div>
          <div class="qr-status">Drag to select the QR code area</div>
        </div>
        <button class="qr-cancel" aria-label="Close scanner">Close</button>
      </div>
      <div class="qr-hint">Release to scan · Esc cancels</div>
    </div>
    <div class="qr-selection" style="display:none"></div>
    <div class="qr-selection-label" style="display:none"></div>
    <div class="qr-result" style="display:none"></div>
    <div class="qr-scan-badge" style="display:none">Scanning</div>
  `;

  document.head.insertAdjacentHTML('beforeend', `<link rel="stylesheet" href="${styleHref}">`);
  document.documentElement.appendChild(overlay);

  const backdrop = overlay.querySelector('.qr-backdrop');
  const selection = overlay.querySelector('.qr-selection');
  const selectionLabel = overlay.querySelector('.qr-selection-label');
  const status = overlay.querySelector('.qr-status');
  const hint = overlay.querySelector('.qr-hint');
  const cancelBtn = overlay.querySelector('.qr-cancel');
  const resultBox = overlay.querySelector('.qr-result');
  const scanBadge = overlay.querySelector('.qr-scan-badge');

  let startX = 0;
  let startY = 0;
  let rect = null;
  let responseListener = null;
  let diagnostics = { steps: [], error: null };

  function removeOverlay() {
    if (responseListener) {
      api.runtime.onMessage.removeListener(responseListener);
      responseListener = null;
    }
    document.removeEventListener('keydown', onKeyDown, true);
    window.__qr_selector_active = false;
    overlay.remove();
  }

  function resetToSelectionMode(message) {
    resultBox.style.display = 'none';
    scanBadge.style.display = 'none';
    setStatus(message || 'Drag to select the QR code area');
    hint.textContent = 'Release to scan · Esc cancels';
  }

  function setStatus(message) {
    status.textContent = message;
  }

  function setBusy(isBusy, message) {
    overlay.classList.toggle('qr-busy', isBusy);
    scanBadge.style.display = isBusy ? 'block' : 'none';
    if (message) setStatus(message);
    try { cancelBtn.disabled = !!isBusy; } catch (e) {}
  }

  function setCaptureUiVisible(visible) {
    overlay.classList.toggle('qr-capture-hidden', !visible);
  }

  function updateSelectionLabel(x, y, w, h) {
    const labelText = `${w}px × ${h}px`;
    selectionLabel.textContent = labelText;
    selectionLabel.style.display = 'block';
    const labelWidth = Math.max(126, Math.min(220, 72 + labelText.length * 7));
    const left = Math.min(x + w + 12, window.innerWidth - labelWidth - 12);
    const top = Math.max(y - 40, 12);
    selectionLabel.style.left = `${Math.max(12, left)}px`;
    selectionLabel.style.top = `${top}px`;
  }

  function onKeyDown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      removeOverlay();
    }
  }

  cancelBtn.addEventListener('click', () => {
    if (overlay.classList.contains('qr-busy')) {
      setStatus('Cannot close while scanning');
      return;
    }
    removeOverlay();
  });
  backdrop.addEventListener('click', (e) => {
    if (overlay.classList.contains('qr-busy')) {
      e.stopPropagation();
      return;
    }
    removeOverlay();
  });

  document.addEventListener('keydown', onKeyDown, true);

  overlay.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    // Ignore clicks on overlay UI controls (result card / HUD / buttons)
    if (e.target && e.target.closest && e.target.closest('.qr-result, .qr-hud')) {
      return;
    }
    resultBox.style.display = 'none';
    scanBadge.style.display = 'none';
    setCaptureUiVisible(true);
    startX = e.clientX;
    startY = e.clientY;
    rect = {x: startX, y: startY, w: 0, h: 0};
    selection.style.display = 'block';
    selectionLabel.style.display = 'block';
    selection.style.left = `${startX}px`;
    selection.style.top = `${startY}px`;
    selection.style.width = '0px';
    selection.style.height = '0px';
    setStatus('Release to scan the selected area');
    hint.textContent = 'You can adjust the box before releasing';
    e.preventDefault();
  });

  overlay.addEventListener('mousemove', (e) => {
    if (!rect) return;
    const x = Math.min(e.clientX, startX);
    const y = Math.min(e.clientY, startY);
    const w = Math.abs(e.clientX - startX);
    const h = Math.abs(e.clientY - startY);
    rect = {x, y, w, h};
    selection.style.left = `${x}px`;
    selection.style.top = `${y}px`;
    selection.style.width = `${w}px`;
    selection.style.height = `${h}px`;
    updateSelectionLabel(x, y, w, h);
  });

  overlay.addEventListener('mouseup', async (e) => {
    try { e.stopPropagation(); e.preventDefault(); } catch (err) {}
    if (!rect) return;
    selection.style.display = 'none';
    selectionLabel.style.display = 'none';
    setBusy(true, 'Capturing and decoding…');
    const capturedRect = rect;
    setCaptureUiVisible(false);
    console.debug('[qr] mouseup capturedRect=', capturedRect);
    responseListener = async (msg) => {
      if (!msg || msg.type !== 'SCREENSHOT_RESULT') return;
      console.debug('[qr] runtime message received', msg && msg.type);
      api.runtime.onMessage.removeListener(responseListener);
      responseListener = null;
      diagnostics.steps.push('received SCREENSHOT_RESULT');
      if (msg.error) {
        setBusy(false, 'Screenshot failed');
        hint.textContent = msg.error;
        setCaptureUiVisible(true);
        return;
      }
      try {
        await handleScreenshot(msg.dataUrl, capturedRect, window.devicePixelRatio || 1);
      } catch (err) {
        setBusy(false, 'Decoding failed');
        hint.textContent = err.message;
        setCaptureUiVisible(true);
      }
    };
    api.runtime.onMessage.addListener(responseListener);
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    diagnostics.steps.push('sending REQUEST_SCREENSHOT');
    console.debug('[qr] sending REQUEST_SCREENSHOT');
    api.runtime.sendMessage({type: 'REQUEST_SCREENSHOT'});
    rect = null;
  });

  async function handleScreenshot(dataUrl, rectCss, dpr) {
    diagnostics = { steps: [], error: null };
    diagnostics.steps.push('start handleScreenshot');
    setBusy(true, 'Preparing image…');
    const img = new Image();
    img.src = dataUrl;
    await img.decode();

    const canvas = document.createElement('canvas');
    const sx = Math.round(rectCss.x * dpr);
    const sy = Math.round(rectCss.y * dpr + window.scrollY * dpr);
    const sw = Math.round(rectCss.w * dpr);
    const sh = Math.round(rectCss.h * dpr);
    canvas.width = sw;
    canvas.height = sh;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh);
    diagnostics.steps.push(`canvas drawn (sw=${sw}, sh=${sh})`);

    // Try BarcodeDetector first
    if (window.BarcodeDetector) {
      try {
        const detector = new BarcodeDetector({formats: ['qr_code']});
        const bitmap = await createImageBitmap(canvas);
        const barcodes = await detector.detect(bitmap);
        diagnostics.steps.push(`BarcodeDetector: found ${barcodes.length} items`);
        if (barcodes && barcodes.length) {
          showResult(barcodes.map(b => b.rawValue).join('\n'), 'Built-in detector', { barcodes });
          return;
        }
      } catch (e) {
        diagnostics.steps.push('BarcodeDetector error: ' + (e && e.message));
        // fall through to fallback
      }
    }

    // Fallback: jsQR. Attempt to load/evaluate vendor/jsQR.js at runtime if missing.
    if (typeof jsQR === 'undefined') {
      const tryLoadFromUrl = async (url) => {
        try {
          const res = await fetch(url);
          if (!res.ok) return false;
          const src = await res.text();
          try { (0, eval)(src); return typeof jsQR !== 'undefined'; } catch (e) {
            try { new Function(src)(); return typeof jsQR !== 'undefined'; } catch (e2) { return false; }
          }
        } catch (e) { return false; }
      };

      // try local vendor file first
      const localUrl = api.runtime.getURL('vendor/jsQR.js');
      let loaded = await tryLoadFromUrl(localUrl);
      if (!loaded) {
        // fallback to CDN for environments where vendor file is missing
        const cdnUrl = 'https://cdn.jsdelivr.net/npm/jsqr@1.4.0/dist/jsQR.js';
        await tryLoadFromUrl(cdnUrl);
      }
    }

    if (typeof jsQR !== 'undefined') {
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      diagnostics.steps.push('Extracted imageData for jsQR');
      const code = jsQR(imageData.data, canvas.width, canvas.height);
      diagnostics.steps.push('jsQR result: ' + (code ? 'found' : 'none'));
      if (code) {
        showResult(code.data, 'jsQR fallback', { jsqr: code });
        return;
      }
    } else {
      diagnostics.error = 'No decoder available';
      setBusy(false, 'No decoder available');
      hint.textContent = 'Reload the extension after adding jsQR.';
      setCaptureUiVisible(true);
      return;
    }

    diagnostics.steps.push('No QR found');
    setBusy(false, 'No QR code found');
    hint.textContent = 'Try a tighter crop around the QR code.';
    setCaptureUiVisible(true);
  }

  function showResult(text, source, meta) {
    const value = (text || '').trim();
    const link = parseUrl(value);
    setCaptureUiVisible(true);
    resultBox.style.display = 'block';
    resultBox.innerHTML = `
      <div class="qr-result-card">
        <div class="qr-result-header">
          <div class="qr-result-title">Result</div>
          <button class="qr-close" aria-label="Close result">×</button>
        </div>
        <pre class="qr-result-text">${escapeHtml(value)}</pre>
        <div class="qr-actions">
          <button class="qr-secondary qr-again">Scan again</button>
          ${link ? '<button class="qr-primary qr-open">Open link</button>' : ''}
          <button class="qr-primary qr-copy">Copy</button>
        </div>
      </div>`;

    const copyBtn = resultBox.querySelector('.qr-copy');
    const closeBtn = resultBox.querySelector('.qr-close');
    const againBtn = resultBox.querySelector('.qr-again');
    const openBtn = resultBox.querySelector('.qr-open');

    copyBtn.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(value);
        setStatus('Copied to clipboard');
      } catch (e) {
        setStatus('Copy failed');
      }
    });

    closeBtn.addEventListener('click', () => {
      removeOverlay();
    });

    againBtn.addEventListener('click', () => {
      resultBox.style.display = 'none';
      resetToSelectionMode('Drag to select the QR code area');
      setCaptureUiVisible(true);
    });

    if (openBtn && link) {
      openBtn.addEventListener('click', () => {
        window.open(link, '_blank', 'noopener,noreferrer');
      });
    }

    setBusy(false, 'QR decoded successfully');
    hint.textContent = link b? 'You can open the link or copy the content.' : 'You can copy the decoded content below.';
  }

  function parseUrl(value) {
    try {
      const url = new URL(value);
      return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null;
    } catch (e) {
      return null;
    }
  }

  function escapeHtml(s){ return (s+'').replace(/[&<>"']/g, c=>({
    '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":"&#39;"}[c])); }

})();
