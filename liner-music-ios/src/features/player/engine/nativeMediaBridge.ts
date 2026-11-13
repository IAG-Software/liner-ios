import { playerEngine } from "../engine/playerEngine";

/**
 * Native media-surface integration.
 *
 * iOS shows Now Playing for apps that publish `MPNowPlayingInfoCenter`. A
 * WKWebView never does that, so the Rust side owns the centre and forwards
 * `MPRemoteCommandCenter` presses back here as `media:remote`. This module is
 * the only place that translates those into engine calls.
 */
export function installNativeMediaBridge(): void {
  if (typeof window === "undefined") return;
  const api = window.linerElectron;
  if (!api?.onRemoteCommand) return;

  api.onRemoteCommand((command) => {
    switch (command) {
      case "play":
        playerEngine.resume();
        break;
      case "pause":
        playerEngine.pause();
        break;
      case "toggle":
        playerEngine.togglePlayPause();
        break;
      case "next":
        void playerEngine.skipNext();
        break;
      case "previous":
        void playerEngine.skipPrevious();
        break;
      case "seek-forward": {
        const { positionMs } = playerEngine.getSnapshot();
        playerEngine.seek(positionMs + 15_000, true);
        break;
      }
      case "seek-backward": {
        const { positionMs } = playerEngine.getSnapshot();
        playerEngine.seek(Math.max(0, positionMs - 15_000), true);
        break;
      }
      default:
        break;
    }
  });

  api.onRemoteSeek?.((seconds) => {
    playerEngine.seek(Math.max(0, seconds * 1000), true);
  });
}
