# zagzig-tools

A Windows network/admin toolkit for settings Windows' own Settings app and Control Panel don't
expose — NRPT rules, per-adapter DNS servers, the WinHTTP proxy, the hosts file,
Authenticode signing, and more. Ships as two independent apps built from one codebase: a desktop
GUI and a terminal UI, both talking to the same underlying Windows APIs and PowerShell cmdlets.

The desktop app is Windows-only by nature — most features shell out to PowerShell cmdlets
(`Get-NetRoute`, `Set-DnsClientServerAddress`, `Get-DnsClientNrptRule`, ...) or Win32/WinAPI
facilities that don't exist on other platforms. The terminal UI's subset of features has no such
dependency and runs on Linux too — see below.

Looking for how to use the app rather than how it's built? See the
[user manual](./docs/user-manual.md) ([PDF](./docs/user-manual.pdf)).

## Features

| Feature | What it does |
| --- | --- |
| Dashboard | A grid of icons for every feature, with starred features kept on top (also at the top of the sidebar) |
| User Manual | This project's user manual, built into the app with a searchable table of contents (English only) |
| Languages | English and Indonesian built in; add any other language as a JSON file — export a template, translate it, import it, no rebuild needed |
| NRPT Rules | View Name Resolution Policy Table rules — what `Add-DnsClientNrptRule` configures |
| Connection Test | Ping + traceroute a host in plain language: is it reachable, and what path did it take; saved hosts shared with Port Scanner and TLS Inspector |
| Network Routes | View and manage the Windows IP routing table (`route print`/`add`/`delete`, with a GUI) |
| Ports | See which application uses which TCP/UDP port, with the owning process's details; spot reserved port ranges (Hyper-V/WSL/Docker) and stop the process or service holding a port |
| Port Proxy | Add, view and remove `netsh interface portproxy` forwarding rules — the usual WSL/Docker workaround, with no built-in GUI |
| Port Scanner | A simple TCP connect scan of one host — which ports accept a connection, and the service they usually carry |
| Wake-on-LAN | Turn on a PC that's off or asleep with a magic packet; saved devices, no software needed on the target |
| Network Adapters | Each adapter's addresses, gateway, DNS, DHCP, link speed and traffic counters, with enable/disable and DHCP renew |
| DNS Servers | Set DNS servers per network adapter — as many as you need, not just preferred/alternate |
| DNS Lookup | A `dig`-style query tool: pick a record type and a specific DNS server to ask |
| DNS Cache | View and flush the resolver cache — what `ipconfig /displaydns`/`/flushdns` do, no GUI |
| DNS Monitor | Background, continuous resolution checks against chosen DNS servers, with a running log |
| Hosts File | Edit `C:\Windows\System32\drivers\etc\hosts`, structured or raw, drag entries to reorder them, and restore from automatic backups — no built-in GUI exists for this |
| SSH Config | Add, edit and remove `Host` entries in `~/.ssh/config`, structured or raw — no admin rights needed |
| WSL | Stop distros, shut down or force-restart a hung WSL (and Docker Desktop), and edit `.wslconfig` |
| Firewall | List Windows Defender Firewall rules, turn them on or off, and create or delete allow/block rules for a port (also from a Ports row) |
| Neighbors (ARP) | View and clear the IPv4 ARP / IPv6 neighbor cache — which devices this PC has talked to recently |
| VPN | See and connect/disconnect the VPN connections built into Windows |
| Wi-Fi | Saved wireless networks and their security; show a saved password after administrator approval |
| Services | List Windows services; start, stop, restart and change their startup type |
| Environment Variables | Edit user and system environment variables, with a list editor for PATH (missing/duplicate folders flagged) and undo for every change |
| Event Log | Recent System/Application events filtered to network & DNS problems and WSL/Hyper-V/Docker |
| Proxy Settings | The WinHTTP proxy (`netsh winhttp`) — the machine-wide proxy Windows Update and many background services and CLI tools honor, separate from the one in Settings > Network |
| Diagnostic Report | One Markdown report of the PC's network setup for a support ticket, with names, MACs and IPs optionally hidden |
| Startup | See and switch off what starts at sign-in (registry Run keys and Startup folders) — the same switch Task Manager uses |
| TLS Inspector | Connect to a host:port and show its certificate, chain, expiry and whether Windows trusts it |
| Code Signing | Sign and verify files with Authenticode via `signtool.exe`, auto-located from the Windows SDK |
| Certificate Store | Browse, view details of, export, import (DER, PEM bundles, PFX) or delete certificates without `certmgr.msc` |

Anything that writes to the system (removing an NRPT rule, adding a route, changing DNS servers,
editing the hosts file, ...) needs administrator rights. The app itself runs unelevated and asks
for a single UAC prompt only for the specific write being made, instead of requiring the whole
app to run elevated all the time.

### Terminal UI (`tui/`)

A standalone `ratatui`-based terminal app (`zagzig-tui`) covering a subset of the same
functionality — Dashboard, Connection Test, DNS Servers, DNS Monitor — for headless boxes,
SSH sessions, or anyone who'd rather stay in a terminal. It's a separate Rust binary with its own
`Cargo.toml`, not a Tauri window; see [`tui/`](./tui).

Runs on **Windows and Linux** — each release ships a `zagzig-tui-x86_64-pc-windows-msvc.zip` and a
`zagzig-tui-x86_64-unknown-linux-gnu.zip` (built by the `release-windows` and `tui-linux` jobs in
`.github/workflows/release.yml` respectively). The platform-specific bits (reading DNS servers,
the permission-denied hint for raw ICMP sockets) are isolated behind `#[cfg(target_os = ...)]` in
`tui/src/sysdns.rs` and `tui/src/ping.rs`; everything else is shared.

### Staying up to date

Both apps check this repo's GitHub releases on startup and can update themselves in place.

## Tech stack

**Desktop app** (`src/`, `src-tauri/`)
- [Tauri 2](https://tauri.app/) (Rust) as the native shell, with the `dialog`, `opener`,
  `updater`, and `process` plugins
- [React 19](https://react.dev/) + TypeScript, built with [Vite](https://vite.dev/)
- [Tailwind CSS 4](https://tailwindcss.com/) and [Base UI](https://base-ui.com/) primitives,
  scaffolded via the [shadcn](https://ui.shadcn.com/) CLI (`components.json`, style `base-nova`)
- [i18next](https://www.i18next.com/) / `react-i18next` for i18n (English and Indonesian today —
  see `src/i18n/locales/`)
- `next-themes` for light/dark mode, `sonner` for toasts, `lucide-react` for icons
- Rust backend commands (`src-tauri/src/lib.rs`) that shell out to PowerShell and `signtool.exe`;
  writes needing admin rights run through a single-UAC-prompt elevation helper so the app itself
  never needs to run elevated

**Terminal UI** (`tui/`)
- [ratatui](https://ratatui.rs/) + [crossterm](https://github.com/crossterm-rs/crossterm) for the
  TUI itself, on a [tokio](https://tokio.rs/) multi-threaded runtime
- [`hickory-resolver`](https://github.com/hickory-dns/hickory-dns) and
  [`surge-ping`](https://github.com/kolapapa/surge-ping) for DNS resolution and ICMP pings
- [`self_update`](https://github.com/jaemk/self_update) for in-place self-updating from GitHub
  releases

**Package managers/tooling**: [Bun](https://bun.sh/) for the frontend, Cargo for both Rust
projects.

## Project layout

```
src/                    React frontend (the desktop app's UI)
  components/           Shared UI (shadcn/Base UI primitives in components/ui, plus app-level bits)
  features/             One folder per sidebar feature (dashboard, dns, hosts, proxy, ...)
  i18n/locales/          en.ts / id.ts translation tables
  lib/                   Hooks and utilities shared across features
src-tauri/               Rust/Tauri backend — PowerShell-backed commands, elevation, updater config
tui/                     Standalone terminal UI binary (separate Cargo project)
docs/                    User manual (user-manual.md, user-manual.pdf) — how to use the apps, not how they're built
.github/workflows/       Release automation (builds + signs the desktop app, builds the TUI for Windows and Linux)
```

## Development

Desktop app:

```sh
bun install
bun run tauri dev     # or: bun run dev (frontend only, in a browser)
```

Terminal UI:

```sh
cd tui
cargo run
```

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Zed](https://zed.dev/) — no extensions needed. Rust (`src-tauri/`, `tui/`) and TypeScript/TSX
  (`src/`) support are both built in, and so is Tailwind CSS autocomplete (bundled
  `tailwindcss-language-server`); there's no Tauri-specific extension, but none is needed since
  `tauri.conf.json` and the capability files are just JSON. The one thing worth adding to your
  `settings.json` is telling the Tailwind server about this project's `cn`/`cva` helpers
  (`src/lib/utils.ts`, `src/components/ui/*`), since without it class names inside those calls
  won't autocomplete:

  ```json
  {
    "lsp": {
      "tailwindcss-language-server": {
        "settings": {
          "classFunctions": ["cva", "cn", "clsx"]
        }
      }
    }
  }
  ```

## License

MIT — see [LICENSE](./LICENSE).
