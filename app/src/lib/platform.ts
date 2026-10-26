/** Desktop OS, from the webview's user agent (WKWebView, WebView2 or WebKitGTK).
 * iPadOS reports a Mac user agent; check `Settings.mobile` for iPad first. */
export type DesktopOs = "macos" | "windows" | "linux";

export const desktopOs: DesktopOs = (() => {
  const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Linux|X11|CrOS/i.test(ua)) return "linux";
  return "macos";
})();

/** What the OS calls its file manager, for "Show in …" actions. */
export const fileManagerName = { macos: "Finder", windows: "File Explorer", linux: "file manager" }[desktopOs];

/** How the pager's USB network link comes up on this OS. */
export const usbLinkHint = {
  macos: "It appears as a network adapter, so macOS and iPadOS need no driver.",
  windows: "It appears as a network adapter. Windows needs Unication's RNDIS driver, which comes with PPS, installed once.",
  linux: "It appears as a network adapter (cdc_ether), so Linux needs no driver; NetworkManager picks up its address.",
}[desktopOs];
