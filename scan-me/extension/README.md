# Select-to-QR Reader (browser extension)

This extension lets you draw a rectangle on the page and attempts to decode a QR code inside the selected area.

Installation (temporary load):

Firefox:
1. Open `about:debugging#/runtime/this-firefox`.
2. Click "Load Temporary Add-on" and choose the `manifest.json` inside `extension/`.

Chrome / Edge:
1. Open `chrome://extensions/` (or `edge://extensions/`).
2. Enable "Developer mode".
3. Click "Load unpacked" and choose the `extension/` folder.

How it works:
- Click the extension toolbar button (action) to activate selection.
- Draw a rectangle over the page to select an area.
- The extension captures the visible tab, crops the selection and attempts to decode.

Decoding behavior:
- If the browser provides the `BarcodeDetector` API, it will be used (fast, native).
- Otherwise the extension will look for a global `jsQR` function (jsQR library). To add jsQR, download a build of jsQR (for example from https://github.com/cozmo/jsQR) and place it in this folder as `jsQR.js` and include it in `manifest.json` under `web_accessible_resources` or load it as a content script.

Notes & next steps:
- This scaffold focuses on selection UI + capture + decode glue. You may want icons and better UX, keyboard shortcuts, and automatic retries.
- For better cross-browser fallback, include `jsQR` file in the extension and wire it as a content script so the fallback works without manual steps.
