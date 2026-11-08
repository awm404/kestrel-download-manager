# Kestrel Companion (browser extension)

A Manifest V3 extension for Chromium browsers (Chrome, Edge, Brave, Vivaldi, Opera) that hands downloads to [Kestrel Download Manager](../README.md) and the aria2 engine behind it.

- **Auto-intercept:** when the browser starts downloading a file with one of the listed extensions, the download is sent to Kestrel instead. The default list covers video files, archives, disk images and installers: `mkv mp4 avi mov wmv flv webm ts m4v zip rar 7z tar gz bz2 xz iso bin exe msi apk dmg pkg deb rpm`. You can edit the list in the popup under **Target File Formats**. If Kestrel is not running or rejects the request, the browser downloads the file as usual.
- **Right-click menu:** **Download with Kestrel** on links, images, audio and video, and **Download Current Page Media with Kestrel** on a page.
- **Popup:** shows whether the engine is reachable, lets you turn auto-intercept on or off, and has a box for sending any URL.

Downloads keep working on sites that need you to be signed in. The extension passes along the browser's cookies for that URL, the referring page and your browser's user agent.

## Install

The extension is not published in a web store yet, so you load it unpacked:

1. In Kestrel, open **Preferences** (<kbd>Ctrl</kbd>+<kbd>,</kbd>) → **Browser Companion Extension** → **Open Extension Folder**. This opens the `kestrel-extension` folder that ships with the app. If you built Kestrel from source, you can also use the `kestrel-extension/` folder in the repository.
2. In your browser, go to `chrome://extensions` (Chrome, Brave, Vivaldi) or `edge://extensions` (Edge).
3. Turn on **Developer mode**.
4. Select **Load unpacked** and choose the `kestrel-extension` folder.
5. Optional: pin **Kestrel Download Manager Companion** to the toolbar.

## Connect it to Kestrel

Kestrel protects aria2 with an RPC secret (see [Security model](../README.md#security-model)), so the extension needs the same secret:

1. Leave Kestrel running and reopen this popup. It reads the secret from Kestrel automatically, and the status should change to **Online**.
2. To copy it by hand instead: **Preferences → Advanced Engine Connection (Remote Daemon) → Copy** next to **RPC Secret Token**, then paste it into **RPC Secret**. It is saved as you type.

The extension talks to `http://127.0.0.1:6800/jsonrpc`, which is Kestrel's default.

## Permissions

| Permission | Why it is needed |
| --- | --- |
| `downloads` | To see when a download starts and cancel the browser's copy once Kestrel has taken it over. |
| `cookies` | To read the cookies the browser would send to the download URL, so signed-in downloads work. |
| `contextMenus` | For the **Download with Kestrel** right-click items. |
| `storage` | To store your settings and RPC secret locally (`chrome.storage.local`). Nothing is synced. |
| `notifications` | To tell you when a download was sent, or when the secret is missing or wrong. |
| Host access `<all_urls>` | Required to read cookies for whichever site you download from, and to reach the local aria2 server. |

## Privacy

- The extension sends data **only to your local aria2** at `127.0.0.1`. It has no analytics and makes no other network requests.
- For each download it sends:
  - the URL
  - the referring page
  - your user agent
  - the cookies your browser would send to that exact URL

  Cookies from other sites, including the page you clicked the link on, are never included.
- aria2 then sends those cookies to the download server, just as the browser would. If that server redirects to another host, aria2 may send the same `Cookie` header there too. Only use Kestrel for sites you trust with your session.
- The RPC secret is stored unencrypted in the browser's extension storage, protected by your Windows account.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Popup says **Offline** | Start Kestrel, or check that aria2 is listening on port 6800. |
| Popup says **Wrong secret**, or a notification asks for the RPC secret | Paste the current secret from Kestrel's Preferences. |
| A download was not intercepted | The file extension may not be on the **Target File Formats** list. Links to web pages (`.html`, `.php`, URLs ending in `/`) are intentionally left to the browser. Use **Download with Kestrel** from the right-click menu instead. |
| You want the browser to handle downloads for a while | Turn off **Auto-Intercept** in the popup. |
