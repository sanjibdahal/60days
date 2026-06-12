const api = typeof chrome !== 'undefined' ? chrome : browser;

// Handle screenshot request from content script
api.runtime.onMessage.addListener((msg, sender) => {
  if (msg && msg.type === 'REQUEST_SCREENSHOT') {
    console.debug('[qr background] REQUEST_SCREENSHOT from', sender && sender.tab && sender.tab.id);
    const senderTabId = sender && sender.tab && sender.tab.id;
    api.tabs.captureVisibleTab(null, {format: 'png'}, (dataUrl) => {
      if (api.runtime.lastError) {
        console.debug('[qr background] captureVisibleTab error', api.runtime.lastError.message);
        if (senderTabId) api.tabs.sendMessage(senderTabId, {type: 'SCREENSHOT_RESULT', error: api.runtime.lastError.message});
      } else {
        console.debug('[qr background] captureVisibleTab success, len=', dataUrl && dataUrl.length);
        if (senderTabId) api.tabs.sendMessage(senderTabId, {type: 'SCREENSHOT_RESULT', dataUrl});
      }
    });
    return true;
  }
});

// Inject content script and CSS when the toolbar button is clicked
if (api.action && api.action.onClicked) {
  api.action.onClicked.addListener(async (tab) => {
    try {
      const tabId = tab.id;
      if (!tabId) return;
      // insert CSS
      if (api.scripting && api.scripting.insertCSS) {
        await api.scripting.insertCSS({target: {tabId}, files: ['selector.css']});
      }
      // inject vendor jsQR if present, then contentScript
      if (api.scripting && api.scripting.executeScript) {
        // try to load vendor/jsQR.js first (if included by user)
        try { await api.scripting.executeScript({target:{tabId}, files:['vendor/jsQR.js']}); } catch(e) {}
        await api.scripting.executeScript({target:{tabId}, files:['contentScript.js']});
      } else {
        // fallback for browsers exposing executeScript on tabs (older API)
        try { await api.tabs.executeScript(tabId, {file: 'vendor/jsQR.js'}); } catch(e) {}
        await api.tabs.executeScript(tabId, {file: 'contentScript.js'});
      }
    } catch (err) {
      console.error('Injection failed', err);
    }
  });
}
