# zagzig-tools User Manual

*A Windows network/admin toolkit, and its cross-platform terminal companion.*

This manual covers both apps in the project:

- **zagzig-tools** — the desktop app (Windows only), with a full sidebar of network and admin
  tools.
- **zagzig-tui** — a terminal app covering a subset of the same diagnostics (Windows and Linux).

---

## Table of contents

1. [Installing](#1-installing)
2. [Getting started with the desktop app](#2-getting-started-with-the-desktop-app)
3. [Desktop app features](#3-desktop-app-features)
4. [Using the terminal UI (zagzig-tui)](#4-using-the-terminal-ui-zagzig-tui)
5. [Staying up to date](#5-staying-up-to-date)
6. [Troubleshooting](#6-troubleshooting)
7. [Getting help](#7-getting-help)

---

## 1. Installing

Both apps are published as GitHub Releases at
`github.com/VladRafli/zagzig-tools/releases`.

### Desktop app (Windows)

Download and run one of:

- `zagzig-tools_<version>_x64_en-US.msi` — standard Windows Installer package
- `zagzig-tools_<version>_x64-setup.exe` — NSIS installer

Either one installs the app and adds a Start Menu shortcut. No administrator rights are needed to
install or to run the app day-to-day — see [Administrator rights](#administrator-rights) below for
when they're actually needed.

### Terminal UI (Windows or Linux)

Download and unzip whichever matches your machine:

- `zagzig-tui-x86_64-pc-windows-msvc.zip` → `zagzig-tui.exe`
- `zagzig-tui-x86_64-unknown-linux-gnu.zip` → `zagzig-tui`

There's no installer — extract the archive and run the binary directly from a terminal
(`.\zagzig-tui.exe` on Windows, `./zagzig-tui` on Linux, after `chmod +x zagzig-tui` if needed).

### Verifying your download

Every release also includes `checksums.txt` — SHA-256 hashes for every other file in the release.
To check a download matches:

- **Windows (PowerShell):** `Get-FileHash -Algorithm SHA256 <file>` and compare the hash by eye
- **Linux/macOS/WSL:** `sha256sum -c checksums.txt`, run from the folder you downloaded into

This is a plain integrity check (did the download complete correctly), not an authenticity
signature — see [Staying up to date](#5-staying-up-to-date) for how each app actually verifies
updates it installs automatically.

---

## 2. Getting started with the desktop app

On first launch you'll see a sidebar on the left (grouped into **Overview**, **Network**, and
**Dev Tools**) and the selected page's content on the right. A header bar at the top shows your
administrator status; the sidebar footer has the update indicator (when relevant), theme switcher,
and language switcher.

### Administrator rights

Some actions need administrator approval — removing an NRPT rule, adding or removing a route,
changing DNS servers, editing the hosts file, changing the WinHTTP proxy, or deleting a certificate
from the machine-wide (`Local Machine`) store.

The app itself **never needs to run elevated**. Instead, each privileged action triggers exactly
one UAC prompt for that specific change — you don't have to relaunch the whole app as
Administrator to use any single feature. Locked controls show a lock icon and a tooltip explaining
why; hovering explains what's needed.

If your Windows account isn't in the local Administrators group at all (a standard user account),
those controls simply stay locked — there's no unelevated attempt or ambiguous error, and you'll
see a badge at the top of the window reading "Standard user" instead of "Administrator". The
Dashboard also shows a banner in this case: **"Running as a standard user"**, with a "Restart as
administrator" button, if you *do* have an account capable of elevating but are running the app
without having chosen "Run as Administrator".

### Theme and language

The sidebar footer has a theme selector (System / Light / Dark) and a language selector. The app
currently ships English and Indonesian translations.

### Checking for updates

See [Staying up to date](#5-staying-up-to-date).

---

## 3. Desktop app features

### Dashboard

The landing page. Shows your signed-in user (and, on a domain-joined machine, directory details
like title, department, and manager pulled over LDAP), an admin-rights banner if applicable, and
summary cards for NRPT Rules, Connection Test, Network Routes, and DNS Servers with an **Open**
button into each.

### NRPT Rules

*Network → NRPT Rules*

Shows the Name Resolution Policy Table — the same rules `Get-DnsClientNrptRule` reports, which
route DNS queries for a given namespace (e.g. `.corp.example.com`) to specific servers. Each rule
card can be expanded ("details") to see every field: DNSSEC settings, DirectAccess settings, IPsec
CA restriction, and so on.

- **Removing a rule** is real and takes effect immediately, with a single UAC prompt if needed.
- **Adding a rule** through the "New rule" form is currently session-only — it's kept in a
  "Pending rules" list in the app but is **not written to Windows**. This is a known limitation,
  not a bug: the form is there to compose a rule's fields, but applying it to the system isn't
  wired up yet.

### Connection Test

*Network → Connection Test*

Enter a hostname or IP and run a test to see:

- **Is it reachable?** — 4 ICMP pings, with average reply time and how many were answered
- **Path it took** — a traceroute-style hop list to the target, with each hop's reverse-DNS
  name shown alongside its address when one exists (`name [ip]`, the same convention
  `tracert.exe` uses) — plenty of hops along the way have no PTR record, so this is normal
  for at least some rows

Both run from a single "Run test" action, and your last several tests are kept in a **History**
list you can re-run or clear.

### Network Routes

*Network → Network Routes*

The Windows IP routing table — what `route print` / `route add` / `route delete` manage from the
command line, built on the modern `NetTCPIP` cmdlets. By default, purely local/system routes are
hidden; toggle **"Show system routes"** to see everything.

- **Add route**: destination (CIDR), next hop, interface, an optional metric, and whether to
  persist the route across a restart. Requires administrator rights and takes effect immediately.
- **Remove route**: per-row delete button (locked without admin rights).

### DNS Servers

*Network → DNS Servers*

Per-network-adapter DNS server configuration — like the "Use the following DNS server addresses"
dialog in adapter properties, except that dialog only offers a preferred and an alternate server;
this lets you set as many as you need per adapter.

- **Edit**: opens a dialog to add/remove/reorder servers for that adapter (order is the order
  they're tried). Requires administrator rights.
- **Reset to automatic**: switches the adapter back to DHCP-provided DNS servers.

### DNS Cache

*Network → DNS Cache*

The resolver cache — what `ipconfig /displaydns` shows and `ipconfig /flushdns` clears, with no
GUI anywhere in Windows for either. Lists every cached record (name, type, data, remaining TTL),
including negative-cache entries (a lookup that came back empty, shown with no data and a status
like "No records of this type").

- **Flush DNS cache**: clears the entire cache. Unlike every other write in this app, this does
  **not** need administrator rights — flushing the client resolver cache is allowed from a
  standard session, so there's no UAC prompt here.

### DNS Monitor

*Network → DNS Monitor*

A background watcher: add a hostname (optionally against a specific DNS server; leave blank for
the system default) and a check interval (1 second up to 30 minutes), and it repeatedly resolves
that hostname and logs whether it succeeded, how long it took, and what addresses came back — even
while you're on a different page of the app. Each monitor can be started/stopped independently, and
its log cleared.

### Hosts File

*Network → Hosts File*

Edits `C:\Windows\System32\drivers\etc\hosts` — the file behind every "add this to your hosts
file" troubleshooting guide, which has no dedicated GUI anywhere in Windows.

- **Structured view**: a table of entries (enabled toggle, IP, hostnames, comment) with per-row
  enable/disable and delete, plus an "add entry" form. All of these require administrator rights
  and take effect immediately.
- **Raw editor**: an expandable text editor showing the whole file as-is, for anything the
  structured view doesn't understand. Saving replaces the entire file and requires administrator
  rights.

### Proxy Settings

*Network → Proxy Settings*

The **WinHTTP** proxy (what `netsh winhttp show proxy` reports) — a separate, machine-wide setting
from the proxy under Settings → Network. Windows Update's underlying service and many background
agents and CLI tools only honor this one, which is why it's easy to set the "wrong" proxy and have
some things still fail.

- View the current WinHTTP proxy (direct access, or a server + bypass list).
- **Set proxy**: server address and optional bypass list. Requires administrator rights.
- **Reset to direct access**: clears it back to no proxy.
- **Import from system proxy**: copies whatever's configured under Settings → Network → Proxy into
  WinHTTP — the quick fix when a tool ignores the proxy you already set elsewhere.

### Code Signing

*Dev Tools → Code Signing*

A wrapper around `signtool.exe` from the Windows SDK (not part of Windows itself — the page first
locates it, auto-detecting common Windows Kits install paths, or falling back to whatever's on
`PATH`; you can also locate a copy manually). Unlike the network features, this page isn't gated
by administrator rights — it's gated only on whether `signtool.exe` was found.

- **Sign a file**: pick a file, then either a certificate from your personal certificate store
  (`CurrentUser\My`) or a `.pfx`/`.p12` file and its password, plus digest algorithm (SHA256/SHA1),
  an optional timestamp server, and an optional description. Output (signtool's own console
  output) is shown inline.
- **Verify a signature**: pick a file and check whether it's signed and trusted.

### Certificate Store

*Dev Tools → Certificate Store*

Browse installed certificates without `certmgr.msc`'s narrow columns and confusing tree. Switch
between Personal, Trusted Root Certification Authorities, Intermediate Certification Authorities,
and Trusted Publishers, each for either the current user or the local machine.

- **View details**: subject, issuer, thumbprint, serial number, friendly name, validity dates,
  whether it has a private key, and its usage.
- **Export**: saves the public certificate (`.cer`) to a location you choose.
- **Delete**: removes the certificate. Only certificates in a **Local Machine** store need
  administrator rights to delete — **Current User** store certificates belong to your own account
  and can be removed without elevation.

---

## 4. Using the terminal UI (zagzig-tui)

Launch it from a terminal: `.\zagzig-tui.exe` (Windows) or `./zagzig-tui` (Linux). It opens a
full-screen menu on the left and the selected screen's content on the right, with a one-line status
bar at the bottom showing keybindings relevant to wherever you currently are.

### Global keys (menu focused)

| Key | Action |
| --- | --- |
| `↑`/`↓` or `j`/`k` | Move between menu items |
| `Enter`, `Tab`, `→`, or `l` | Open the selected section |
| `Esc` (inside a section) | Back to the menu |
| `q` or `Esc` (in the menu) | Quit |
| `u` | Install an available update, or retry after an error (see below) |
| `r` | Restart after an update has installed |

### Screens

- **Dashboard** — a summary: how many DNS monitors are running, how many DNS server groups were
  last read, and your last connection test result.
- **Connection Test** — type a host and press Enter to ping it 4 times; results and a short
  history are shown inline.
- **DNS Servers** — read-only view of DNS servers per adapter/link (Windows: parsed from
  `ipconfig /all`; Linux: `resolvectl status` when available — most modern distros — falling back
  to `/etc/resolv.conf` otherwise, since on a systemd-resolved system that file only points at a
  local stub resolver, not the real servers). Press `r` to refresh.
- **DNS Monitor** — `Tab`/`Shift+Tab` to move between the hostname field, server field, interval
  selector, and the monitor list; `←`/`→` change the interval while it's focused; `Enter` adds a
  monitor from the form, or starts/stops the selected one from the list; `x` removes the selected
  monitor; `c` clears its log.

### Linux-specific notes

Reading raw ICMP pings needs elevated permissions on Linux — if Connection Test reports permission
denied, either run as root, grant the binary `CAP_NET_RAW`, or allow unprivileged ping sockets with
`sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"`. On Windows, the equivalent situation
(rare) would ask you to run the terminal as Administrator instead.

---

## 5. Staying up to date

Both apps check this repo's latest GitHub release on startup, again automatically every hour in
the background, and any time you ask them to.

**Desktop app**: the sidebar footer always has an update control. Most of the time it reads
"Check for updates" — click it to check on demand (you'll get a toast either way: up to date, or
couldn't check). When a newer version exists, that same spot turns into a prominent button showing
the version number; click it to see release notes and an "Install and restart" button, which
downloads the update, verifies it against a signing key baked into the app (via Tauri's updater
plugin — cryptographically signed, not just downloaded over HTTPS), installs it, and restarts.
Background checks (startup and hourly) stay silent unless they find something — no toast spam.

**Terminal UI**: press `u` at any time from the menu to check for updates on demand (shown in the
status bar hint). The status bar shows `update available: vX.Y.Z   u: install and restart` when one
exists. Press `u` to download and verify it (signed with a separate Ed25519 key via
[zipsign](https://github.com/Kijewski/zipsign) — a different mechanism from the desktop app's, but
the same idea: the download is rejected if it isn't signed by the matching key, not just trusted
because it came from GitHub). Once installed, the bar changes to `updated to vX.Y.Z — r: restart
now` — press `r` to relaunch.

If a check or install fails, the status bar shows the error with a `u: retry` hint.

---

## 6. Troubleshooting

**A button is locked with a padlock icon.** That action needs administrator rights your current
session doesn't have active. Hover it for the specific reason, or see
[Administrator rights](#administrator-rights).

**"Not found" for signtool.exe.** It ships with the Windows SDK or Visual Studio Build Tools, not
Windows itself. Install one of those, or use "Locate signtool.exe" to point at an existing copy
manually.

**A DNS Monitor entry keeps failing to resolve.** Check the server field — if you specified one and
it's unreachable or doesn't serve that record, resolution will fail even though the hostname itself
is valid. Leave it blank to fall back to your system's default resolver.

**NRPT "New rule" doesn't seem to do anything.** That's expected right now — see the note under
[NRPT Rules](#nrpt-rules) above. Only removing an existing rule is currently wired up to Windows.

**The TUI reports a permission error on Connection Test (Linux).** See
[Linux-specific notes](#linux-specific-notes) above.

**An update fails to verify/install.** Retry with `u` (TUI) or reopen the update dialog (desktop) —
transient network issues during download are the most common cause. If it persists, download the
release directly from GitHub instead and check it against `checksums.txt`.

---

## 7. Getting help

- Issues and questions: `github.com/VladRafli/zagzig-tools/issues`
- Project overview and technical details: [`README.md`](../README.md) in the repository root
- License: MIT — see [`LICENSE`](../LICENSE)
