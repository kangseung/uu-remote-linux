# AGENTS.md — guide for AI coding agents

This file is written for large language model agents (Claude Code, Codex, Cursor, ...) that are asked to
**build, install, debug or develop** uu-remote-linux. Read it fully before acting. Human documentation is in
[README.md](README.md) (Chinese) and [README.en.md](README.en.md).

## 1. What this project is

- An **unofficial Linux host** for NetEase UU Remote (网易 UU 远程). The official Mac / Windows / mobile
  clients connect to it and control the Linux desktop. There is no viewer/controller side here.
- One Rust binary, `uuremote-server` (Cargo package `uu-remote-linux`), plus small C engines in `c_engine/`
  compiled by `build.rs`. No Wine, no NetEase binaries.
- The host registers with the UU cloud as a **Windows PC** (`x-param-plat=1`) so the official clients accept it.
- License GPL-3.0. Not affiliated with NetEase.

## 2. Ground rules (must follow)

1. **Privacy in logs.** Never log keystrokes, terminal text, clipboard contents, file contents, passwords,
   tokens, QR login URLs or TURN credentials. Log sizes, counts, field numbers and booleans only. Existing code
   follows this; keep it that way in every new log line.
2. **The QR login URL is a credential.** Show the QR only to the human who owns the account. Never paste the
   URL or a QR screenshot into chats, issues, logs or commits.
3. **Follow the official host behavior.** Protocol behavior is copied from the official hosts (Mac
   `UURemoteServer`, Windows `GameViewerServer.exe`). Do not invent message shapes; when unsure, log the field
   numbers of what the viewer sends and derive behavior from evidence.
4. **No NetEase assets.** Do not add NetEase logos, icons, screenshots of their apps or copies of their binaries.
5. **No personal data in source.** Nothing machine- or account-specific is compiled in; credentials come from
   the config file / environment. The only built-in default is the public client signing key in `src/auth.rs`.
6. **Don't break other tools.** The desktop, file-transfer, terminal and port-mapping tools each open their own
   peer connection; a new offer must never close other peers.
7. Keep changes small and tested; match the surrounding code style (dense, comment only the non-obvious).

## 3. Build and test

Dependencies (Ubuntu/Debian):

```sh
sudo apt install build-essential pkg-config \
    libx11-dev libxext-dev libxtst-dev libxfixes-dev libavcodec-dev libavformat-dev libavdevice-dev libswscale-dev libclang-dev libgtk-3-dev ffmpeg
# --features inproc-av:      libavcodec-dev libavformat-dev libavdevice-dev libswscale-dev libclang-dev
#                            (FFmpeg 6.x; in-process capture/encoding, Ubuntu 24.04 ships it)
# --features wayland:        libwayland-dev libxkbcommon-dev wl-clipboard
# --features wayland,portal: + libglib2.0-dev libpipewire-0.3-dev
```

Rust stable via rustup. Commands:

```sh
cargo test                                   # unit tests (macOS builds use C stubs, also fine for development)
cargo build --release                        # X11 host
cargo build --release --features wayland     # + Hyprland/wlroots
cargo build --release --features wayland,portal   # + GNOME/KDE Wayland via xdg-desktop-portal
```

- `build.rs` links X11/Xext/Xtst/Xfixes and `dl` (PAM is loaded at runtime with `dlopen`), and builds the GTK3
  login window only when `pkg-config gtk+-3.0` succeeds (`UU_NO_GTK=1` forces it off).
- The portal feature panics in `build.rs` if GIO/PipeWire development packages are missing.
- `vendor/webrtc-dtls` is a patched copy of the upstream crate (MIT/Apache); keep it as a path dependency.
- CI (`.github/workflows/ci.yml`) runs `cargo test --locked` on ubuntu-22.04. Pushing a `v*` tag runs
  `.github/workflows/release.yml`, which builds and publishes the x86_64 tarball; release notes come from
  `docs/release-notes/<tag>.md`.

## 4. Install on a user's machine (step by step)

On Debian/Ubuntu/LMDE the recommended install is the deb from Releases (or `packaging/deb/build-deb.sh`),
which ships the binary, the uinput udev rule and a `--global`-enabled user unit; the steps below are the
from-source equivalent.

1. Check the session: `echo $XDG_SESSION_TYPE $XDG_CURRENT_DESKTOP $DISPLAY $WAYLAND_DISPLAY`.
   X11 → default build. Hyprland → `--features wayland`. GNOME/KDE on Wayland → `--features wayland,portal`
   (or advise the user to pick the "on Xorg" session at the login screen, which is the best-supported path).
2. Install dependencies and build (section 3). Copy the binary: `install -Dm755 target/release/uuremote-server ~/.local/bin/uuremote-server`.
3. **Login (needs the human).** Run `uuremote-server login` inside the graphical session. A GTK window shows a QR
   code; the user scans it with the UU Remote mobile app and confirms. Without a display use
   `uuremote-server login --no-gui` (terminal QR). Credentials are written to `~/.config/uuremote/config.json`
   (mode 0600). Do not read the token out of that file or print it.
4. Create a systemd user unit (`~/.config/systemd/user/uuremote-native.service`):

   ```ini
   [Unit]
   Description=UU Remote Linux host (unofficial)
   After=graphical-session.target
   [Service]
   Environment=RUST_LOG=info
   Environment=DISPLAY=:0
   # GNOME on Xorg usually needs: Environment=XAUTHORITY=/run/user/<uid>/gdm/Xauthority
   ExecStart=%h/.local/bin/uuremote-server
   Restart=always
   RestartSec=5
   [Install]
   WantedBy=default.target
   ```

   Find the real display/Xauthority with `loginctl show-session <id> -p Display` / `ps -o args -C Xorg`.
5. `systemctl --user daemon-reload && systemctl --user enable --now uuremote-native.service`
6. Verify: `journalctl --user -u uuremote-native.service -f` must show
   `Device status synchronized: ... availability=Some("ready")`. If credentials are missing or expired the
   service opens the QR window on the desktop by itself (disable with `UU_LOGIN_WINDOW=0`).
7. Ask the user to connect from the official client and check desktop, input, sound, file transfer, terminal.

Updating an installed binary: copy to a temp name and `mv -f` over the old one (a running binary cannot be
overwritten in place: "text file busy"), then restart the service. Restarting kills open viewer windows; the
viewer does not reconnect them automatically.

## 5. Runtime configuration

| Item | Meaning |
|---|---|
| `~/.config/uuremote/config.json` | user_id, device_id, client_id, system_id, machine_guid, token, csrf; optional `sign_key`, `ffmpeg_root`. `$UU_CONFIG` overrides the path |
| `~/.config/uuremote/host_settings.json` | persisted 安全 settings: auto_lock, keep_mute, mute_permanent |
| `UU_DESKTOP_BACKEND` | `auto` (default) / `x11` / `wayland` / `portal` / `kms` (experimental feature) / `none` (headless: no display; desktop offers ignored; tool peers get a nominal 1920x1080 screen + Allow permissions, which the port-mapping viewer requires) |
| `UU_INPUT_BACKEND` | `auto` (uinput on Wayland/portal when `/dev/uinput` is accessible, udev `uaccess` rule in README) / `uinput` / `native`. X11 keeps XTest (held-key autorepeat handling). With the portal backend and accessible uinput, capture opens a ScreenCast-only portal session instead of RemoteDesktop (KDE's RemoteDesktop dialog has no screen picker; ScreenCast does) |
| `UU_TERMINAL_SHORTCUTS=0` | disable Ctrl+V → Ctrl+Shift+V in focused X11 terminals (Mac Cmd and Ctrl both arrive as VK_LCONTROL) |
| `contrib/virtual-desktop/` | Xvfb + session (`UU_VIRTUAL_SESSION`, `UU_VIRTUAL_BUS=user|private`) for monitorless hosts; serve it with `DISPLAY=:99` |
| `UU_VIDEO_ENCODER` | `cpu`: libx264; `vaapi`: explicit Intel/AMD; `nvenc`: force; unset: NVENC→CPU only if `/proc/driver/nvidia/version` or `/dev/nvidia0` exists, else CPU (#10) |
| `UU_FFMPEG_ROOT` | FFmpeg prefix (`bin/ffmpeg`, libs in `lib/x86_64-linux-gnu`) |
| `UU_AUDIO=0`, `UU_AUDIO_SOURCE` | disable sound / choose the Pulse source (default `@DEFAULT_MONITOR@`) |
| `UU_LOGIN_WINDOW=0` | don't auto-open the login window |
| `UU_STREAM_LOCK_SCREEN=1` | keep streaming the lock screen instead of blanking (KDE keeps the PipeWire stream alive on lock; input via uinput still reaches the lock screen). Default off = privacy. Either way the portal session is rebuilt on the lock→unlock edge (KDE's stream stays "alive" but can silently stop delivering real frames after unlock) and when a stream delivers no frame for 10 s while unlocked. On the portal backend `keep_display_awake` calls `org.freedesktop.ScreenSaver.SimulateUserActivity` (connection start, every 30 s while streaming, on unlock) so DPMS-off outputs wake up |

Token has no expiry field; API code 1120 (HTTP 422) means invalid/expired → re-login. The config is re-read
every 60 s.

## 6. Code map

| Path | Role |
|---|---|
| `src/main.rs` | entry, subcommands, room loop (one peer per `ice_id`), room renewal (deferred while connected) |
| `src/signaling.rs` | Engine.IO v4 / Socket.IO client; `be-controlled` binary attachment = ConnectOptions (field 1 capture_type) |
| `src/auth.rs`, `src/config.rs`, `src/login.rs`, `src/login_ui.rs` | signed REST client, config, device init + QR login, GTK window |
| `src/webrtc.rs` | peer connection, data channels, CONTROL dispatch, initialization messages |
| `src/video.rs`, `src/capture_setting.rs` | FFmpeg H.264 pipeline; viewer quality/fps/resolution settings |
| `src/audio.rs` | Pulse/PipeWire monitor → Opus audio track |
| `src/input.rs`, `src/input_mapping.rs`, `src/uinput.rs` | input injection (X11 XTest / Wayland / portal / uinput), Windows VK → keysym → evdev |
| `src/cursor.rs` | pointer shape mirroring (X11) |
| `src/clipboard*.rs` | text clipboard bridge and protocol |
| `src/file_trans.rs` | FileTransferFtp RPC |
| `src/terminal.rs` | TERM frames + PTY sessions |
| `src/session_lock.rs`, `c_engine/pam_check.c` | lock state reporting, auto unlock via PAM + logind |
| `src/system_actions.rs`, `src/system_metrics.rs` | SimpleAction system commands, mute, auto-lock, SystemMetrics |
| `src/initialization.rs`, `src/control_protocol.rs`, `src/protocol.rs` | protobuf encoders/decoders for CONTROL |
| `src/session_ice.rs`, `src/network_diagnostics.rs` | cloud ICE servers, address-free path diagnostics |
| `src/desktop_backend.rs`, `src/wayland.rs`, `src/portal*.rs` | backend selection, Hyprland, GNOME/KDE portal |
| `c_engine/x11_engine.c`, `c_engine/clipboard.c` | X11 capture/input/selection |
| `c_engine/wayland_engine.c`, `c_engine/portal_engine.c` | Wayland and portal engines |
| `c_engine/login_window.c` | GTK3 login window |

Diagnostic subcommands: `login [--no-gui]`, `unlock-check` (password on stdin, prints only accepted/rejected),
`desktop-probe [new.png]`, `desktop-video-probe`, `wayland-input-probe`, `clipboard-acceptance`,
`portal-acceptance`, `help`. Unknown arguments exit with code 2. Builds up to v0.1.0 did **not** reject them and
started a second full host server instead, so check that a subcommand exists in the binary you run.

## 7. Protocol facts (verified against the official clients)

- Connection types (ConnectOptions.capture_type): 1 desktop, 5 file transfer, 8 terminal, 9 port mapping.
  Video is sent only for desktop connections (capture_type ≤ 2).
- CONTROL messages are protobuf `Message {1 seq, 2 timestamp, <which> payload}`; input arrives as JSON
  (`kbd_press`, `mouse_move_absolute` with normalized 0..1 floats, ...).
- Android IME typing arrives as JSON `{"action":"text_input","content":"..."}` (one message per IME commit;
  observed on the official Android viewer). Injection: printable ASCII → per-char VK keystrokes
  (`input_mapping::ascii_to_windows_vk`, Shift wrapped per char); any non-ASCII → wl-clipboard paste fallback
  (save `wl-paste` text → `wl-copy` the content → Ctrl+V → ~300 ms → restore), gated on WAYLAND_DISPLAY.
  Content capped at 4 KiB; never log the text (counts/booleans only).
- Echo: the viewer sends SimpleAction action 0 with its FeatureFlag; the host answers action 1 with args
  `{"seq":<request seq>}` and **its own** FeatureFlag. The viewer only uses features the host declares:
  `1 ff_capture_setting=4`, `2 ff_simple_action=2`, `3 ff_system_metrics=2`, `6 ff_file_transfer_ftp=2`,
  `8 ff_clipboard=3` (when the clipboard bridge is ready). Do not declare versions you don't implement.
- RPC = Message 21 `RpcRequest {1 header{1 request_id}, <case>}`, answered with Message 22
  `RpcResponse {1 header (same id), <case>}`. Cases used: 2 capture_setting, 3 fps, 4 video_quality,
  6 resolution, 8 file transfer, 9 clipboard, 15 send_video_track, 16–20 plugins, 21 virtual audio policy,
  28/29 terminal.
- CaptureSettingRequest: field 1 is an **fps option index** (1=30, 2=60, 3=90); the real frame rate is
  field 18 `fpsCount`. Field 2 quality: 1 Fast, 2 General (高清 2M), 3 HD (超清 8M), 4 Bluray (原画 20M),
  5 Auto, 6 Custom. Resolution: 16/17 pixel size or 5/6, 10 resolution_type (3 = follow remote).
  Reply `RpcResponse{2 CaptureSettingResponse{}}` (no errors = applied) and send Message 25 CaptureSettingSync.
- SystemMetrics (Message 13): GeneralMetric `2 system_sound_support` is an enum, **2 = supported** (1 = not
  supported); SettingMetric 1 keep_mute, 2 mute_permanent, 3 auto_lock, 5 file_transfer_save_location,
  8/9/10 permission states (2 = allow).
- Security menu: 静音运行 on → SimpleAction 14 + 18, off → 16. 远程结束后自动锁屏 → SimpleAction 19 with
  `value` (field 5) **2 = on, 1 = off**. The viewer sends these only when toggled, so the host persists them.
- Auto unlock: host reports `SystemStateChange{1 SecureDesktopState{1 is_locked, 5 is_support_auto_logon}}`;
  the viewer (after the user enables 自动解锁被控端) sends SimpleAction 15 with args `{"key": password}`;
  failure → Message 12 `ReportError{2: -5}`.
- File transfer: the viewer sends RPCs on CONTROL but receives only on FILE_DATA_CHANNEL, so all replies go
  out on FILE. With `ff_file_transfer_ftp=2` directory listings are zlib-compressed protobuf. `remove_dir` is
  sent for every delete and must be answered with response case **6 `result{id, file_error}`**, not
  `operation_result`. Host root is shown as ":/" (此电脑); home is "~/".
- Terminal: BINARY channel, RPC 28 check_env → 24; frames `"TERM"`, version 1, type (enum+1), 2 flag bytes,
  session u32 LE, length u32 LE, payload; JSON `{cols,rows,shell,option}`.
- Port mapping (capture_type 9): Message 27 `PortMappingFrame {1 session_id, 2 rule_id, 3 stream_id, 4 type, 5 payload}`
  on FILE_DATA_CHANNEL only. Types SYN=0, FIN=1, DATA=2, SYN_ACK=3, DATA_ACK=4 (ignored by both sides). SYN payload
  JSON `{"version","target_host","target_port"}` → connect → SYN_ACK `{"version","ok","error"}`; DATA raw bytes both ways.
  Echo session/rule/stream ids with their original wire types. The viewer does not support TCP half-close.
- SCTP message size: webrtc-rs omits `a=max-message-size`, so the viewer assumes 64 KB and refuses larger messages; the
  answer sent to the viewer adds `a=max-message-size:262144` (only the sent copy — webrtc-rs rejects a modified local
  description) and `vendor/webrtc` raises the data-channel receive buffer from 65535 to 262144 bytes.
- Cursor shape = Message 15 SystemStateChange field 2. Audio: the viewer always offers an Opus m-line.
- Login API: `/api/v1/device/windows/init` → device_id; `/qrcode/gen/login`; long-poll
  `/qrcode/login/status` (1 wait, 2 scanned, 4 confirmed); `/login/by_qrcode` → user token. QR expires after
  ~5 min with HTTP 400 code 1190 (not fatal: generate a new one).

## 8. Known pitfalls

- webrtc-rs `Sample.duration` sets the gap to the **next** frame. FFmpeg 6.1 rejects `-r` together with
  `-fps_mode vfr`.
- All-intra 60 fps saturates Wi-Fi; keep GOP ≈ 0.5 s. A bitrate cap alone does not change quality on a static
  desktop, so presets also change the CQ level.
- X11 capture on a GPU saturated by CUDA work starves; prefer the iGPU for the desktop when available.
- Remote keys repeat at high RTT because key-up arrives after the X autorepeat delay. XTEST repeats through the
  master keyboard; the engine disables global autorepeat only while a remote key is held.
- Interactive shells ignore SIGTERM: the server HUPs/KILLs PTY sessions on shutdown.
- An explicit `DISPLAY` that cannot be opened is an error; the X11 engine falls back to `:0`/`:1` only when DISPLAY is unset.
- systemd/D-Bus-activated apps (GNOME Terminal, portals) run with the user manager's environment, not the caller's:
  a private `dbus-run-session` bus does not bring them onto a virtual display.
- GTK windows stop repainting while GNOME is locked or the monitor is off; test UI on Xvfb or after unlock.
- `XInitThreads()` runs in a library constructor in `x11_engine.c`, never lazily: the login flow's GTK thread
  opens its own X connection before `x11_init()`, and a late call leaves libX11 unlocked → concurrent Xlib use
  from GTK + tokio threads segfaults (observed on Mint 21/Cinnamon, 3/3 repro on first login).
- logind `unlock-session` by the session owner clears the GNOME shield in ~3 s.
- GNOME closes the portal session on lock; capture/input must share one portal session and be rebuilt after
  unlock. Portal input/clipboard permissions are persisted per desktop (0600 file); a revoked permission asks again.
- The uinput absolute tablet maps 0..=65535 onto the whole desktop bounding box, so single-screen ScreenCast
  capture would offset clicks. `c_engine/output_layout.c` enumerates logical outputs via xdg-output; the portal
  stream `position` (KDE provides it) or a unique logical-size match feeds `input_mapping::AbsoluteMapping`
  (`capture.rs` → `InputInjector::set_absolute_mapping`, cleared on portal unbind). Identity when geometry is
  unknown/ambiguous or the whole desktop is captured; logical (not pixel) coordinates everywhere.
- Portal session kinds: `uu_portal_open_persist` (RemoteDesktop: input + clipboard consent) and
  `uu_portal_open_screencast` (ScreenCast-only: capture consent with per-screen picker on KDE, no SelectDevices,
  no portal clipboard; input/clipboard FFI report unsupported). `src/capture.rs` picks ScreenCast when
  `uinput::wanted(backend) && uinput::probe()`; restore tokens use a separate `<desktop>-screen` cache key, and the
  text clipboard falls back to the wl-clipboard bridge (`clipboard::start_wayland`) for ScreenCast sessions.
  `InputInjector` opens uinput eagerly at startup and retries lazily in `bind_portal`, because the systemd
  user service can start before the Wayland socket exists and initially detect X11 (which skips uinput).

## 9. Debugging checklist

- Connection: `Offer for connection: capture_type=`, `PeerConnection State changed`, `ICE path: ... route=`.
- Video: `Desktop encoder: ... fps= cq= maxrate=`, `Desktop latest-frame stats`, `Video feedback totals`.
- Viewer requests: `CONTROL message: fields=[..] simple_action=..` (numbers only), `Capture setting request`,
  `SimpleAction N (...) received`.
- Features: `File transfer ...`, `Terminal frame`, `Native desktop clipboard text received: bytes=`,
  `Desktop audio started`, `Auto unlock result`, `Auto lock after remote session`.
- When the viewer "does nothing" after a menu click, first check whether any message arrived; usually the
  host did not declare the matching FeatureFlag/SystemMetrics capability.

## 10. Contributing

- Run `cargo test` (and the feature builds you touched) before committing; keep commits focused.
- New protocol behavior: cite the evidence (observed field numbers, official log strings) in a comment.
- Update README.md / README.en.md when user-visible features or limitations change, and add
  `docs/release-notes/<tag>.md` before tagging a release.

### VAAPI (opt-in / 显式启用)

`UU_VIDEO_ENCODER=vaapi` selects Intel/AMD H.264 VAAPI; `UU_VAAPI_DEVICE` defaults to
`/dev/dri/renderD128` (render-node permissions required). No automatic VAAPI selection.
Both FFmpeg process and `--features inproc-av` upload NV12 frames to a VAAPI frame pool.
VBR uses the viewer bitrate preset and CQ-derived minimum QP, no B-frames, async depth 1,
AUD and the existing GOP/VFR timing. Unsupported device/driver/initialization falls back to libx264
with a warning. Accepted on Intel UHD 770 (iHD) with the default FFmpeg process pipeline and the official Mac
viewer (presets, fps, scaling, reconnect; no fallback, ~40% less CPU than libx264). AMD and the `inproc-av`
VAAPI path are not hardware-verified.

### Virtual audio / 虚拟声卡

The viewer's 静音运行 switch creates the null sink `uu-remote-sink`,
moves streams from the original default output and captures `uu-remote-sink.monitor`.
New output goes to this sink; streams explicitly pinned to other outputs are untouched.
The last desktop session restores the previous default output and unloads the module.
`mute_permanent` retains the physical mute policy, but still releases the virtual sink.
A private recovery record beside the config and a process lock recover crash leftovers on startup.
`pactl` is used when installed (PulseAudio / PipeWire-pulse); plain PipeWire systems without it (default
Ubuntu desktop) use `pw-cli`/`wpctl` for the same sink and WirePlumber moves playing streams. Accepted on
Ubuntu 24.04 with the official Mac viewer. When routing is unavailable, the existing mute/monitor path remains.
Changing the switch restarts only the scoped audio encoder; normal non-muted capture is unchanged.
`virtual-audio-probe` temporarily routes output for five seconds and restores it (local diagnostic).

### Virtual desktop user bus

The supervisor imports DISPLAY and XAUTHORITY into the linger user bus, waits for authenticated
X11 readiness, and keeps its cleanup trap alive. The sample drop-in uses `%t/uu-virtual-99.Xauthority`.
Use a dedicated headless account. See the [setup and official Mac acceptance checklist](contrib/virtual-desktop/ACCEPTANCE.md).

### Arch Linux / Omarchy

```sh
sudo pacman -S --needed base-devel rust pkgconf clang ffmpeg libx11 libxext libxtst libxfixes gtk3 \
  wayland libxkbcommon wl-clipboard libpipewire glib2 libpulse
cargo test --locked --features wayland
cargo build --release --features wayland
```

Omarchy uses Hyprland: run in its user session, select `UU_DESKTOP_BACKEND=wayland`,
and retain `WAYLAND_DISPLAY` / `XDG_RUNTIME_DIR`. This requires wlr-screencopy and the
virtual input protocols supported by Hyprland; other compositors may need `--features wayland,portal`.
For uinput, load `uinput` and apply the uaccess rule above; seat ACLs require an active login.
For unattended accounts, explicitly restrict `/dev/uinput` access to a dedicated group.
This permission can inject into the whole seat, including lock/greeter screens.
`UU_INPUT_BACKEND=uinput` requests kernel devices; verify startup actually reports uinput.
Optional VAAPI: `intel-media-driver` (modern Intel) or `libva-mesa-driver` (AMD), `libva-utils`
for `vainfo`. Optional KMS: `libdrm`. Virtual Xfce: `xorg-server-xvfb xorg-xauth xorg-xdpyinfo
xfce4-session xfce4-panel xfdesktop xfwm4 gnome-terminal`.
Rolling Arch FFmpeg may be newer than supported `inproc-av` (FFmpeg 6.x); use the default process
pipeline or build against a separate FFmpeg 6 prefix. Do not downgrade the system FFmpeg.
User-bus headless setup and the official Mac checklist: [virtual desktop acceptance](contrib/virtual-desktop/ACCEPTANCE.md).

### Experimental KMS / 实验后端

`--features kms` plus explicit `UU_DESKTOP_BACKEND=kms` enables a limited linear-framebuffer
prototype; `UU_KMS_DEVICE=/dev/dri/card0`. Requires libdrm headers and deliberately granted
`cap_sys_admin+ep`, plus card-node access. It is never automatically selected or granted privileges.
Read the [KMS limitations, permission risk and revocation steps](docs/validation/kms.md) before use.
The VM GDM greeter capture was solid black (failed visual acceptance); lock/greeter support and Intel/AMD hardware remain unverified. Use uinput for input; clipboard
is unavailable without a desktop session. No default release behavior changes.
For headless desktop offers, [official error evidence remains incomplete](docs/validation/headless-error-evidence.md);
the existing offer behavior is retained.
