/**
 * Boot guard — runs before the app bundle, outside the CSP'd module graph.
 *
 * Purpose: turn an invisible failure into something the user can read on the
 * phone. It
 *   1. paints a visible status line so a blank WebView is distinguishable from
 *      a WebView that never loaded,
 *   2. catches `error` / `unhandledrejection` / failed module loads and prints
 *      the message on screen,
 *   3. forwards each message to the native boot log
 *      (`Documents/liner-boot.log`) via the `boot_log` Tauri command.
 */
(function () {
  "use strict";

  var messages = [];

  function logFile(line) {
    try {
      var internals = window.__TAURI_INTERNALS__;
      if (internals && typeof internals.invoke === "function") {
        internals.invoke("boot_log", { message: "[web] " + line });
      }
    } catch (err) {
      /* logging must never throw */
    }
  }

  function render(message, fatal) {
    messages.push(message);
    logFile(message);

    var host = document.getElementById("boot-status");
    if (!host) {
      host = document.createElement("div");
      host.id = "boot-status";
      host.setAttribute(
        "style",
        [
          "position:fixed",
          "inset:0",
          "z-index:2147483647",
          "display:flex",
          "flex-direction:column",
          "gap:8px",
          "align-items:center",
          "justify-content:center",
          "padding:28px",
          "box-sizing:border-box",
          "background:#101014",
          "color:#f2f2f7",
          "font:13px/1.5 -apple-system,system-ui,sans-serif",
          "text-align:center",
          "white-space:pre-wrap",
          "word-break:break-word",
        ].join(";"),
      );
      (document.body || document.documentElement).appendChild(host);
    }

    host.textContent =
      (fatal ? "Liner failed to start\n\n" : "Starting Liner…\n\n") +
      messages.join("\n");
  }

  window.addEventListener("error", function (event) {
    var detail =
      (event && event.message) ||
      (event && event.error && event.error.message) ||
      "unknown script error";
    var where = event && event.filename ? " @ " + event.filename + ":" + event.lineno : "";
    render("error: " + detail + where, true);
  });

  window.addEventListener("unhandledrejection", function (event) {
    var reason = event && event.reason;
    render(
      "unhandled rejection: " +
        ((reason && (reason.stack || reason.message)) || String(reason)),
      true,
    );
  });

  // A module graph that fails to fetch surfaces as a `link`/`script` error with
  // no useful message; catch it explicitly.
  window.addEventListener(
    "error",
    function (event) {
      var target = event && event.target;
      if (target && (target.tagName === "SCRIPT" || target.tagName === "LINK")) {
        render(
          "asset failed to load: " + (target.src || target.href || "unknown"),
          true,
        );
      }
    },
    true,
  );

  // If React never marks the document as mounted, say so instead of showing a
  // spinner forever.
  window.setTimeout(function () {
    if (window.__LINER_MOUNTED) return;
    render(
      "React did not mount within 12s. index.html loaded but the module graph " +
        "did not execute.",
      true,
    );
  }, 12000);

  // Paint a visible "booting" state immediately: a blank screen then means the
  // WebView never loaded this document at all.
  function start() {
    render("booting…", false);

    // Once React renders real content, drop the overlay so it cannot swallow
    // touches.
    var poll = window.setInterval(function () {
      var root = document.getElementById("root");
      var mounted =
        window.__LINER_MOUNTED || (root && root.childElementCount > 0);
      if (!mounted) return;
      window.clearInterval(poll);
      var host = document.getElementById("boot-status");
      if (host && host.parentNode) host.parentNode.removeChild(host);
    }, 200);
  }

  if (document.body) start();
  else window.addEventListener("DOMContentLoaded", start);
})();
