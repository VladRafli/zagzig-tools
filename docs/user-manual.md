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
changing DNS servers, editing or reordering the hosts file, changing the WinHTTP proxy, adding or
removing a port proxy rule, enabling, disabling or renewing a network adapter, stopping a Windows
service, force restarting WSL, or deleting a certificate from the machine-wide (`Local Machine`)
store. Reading the Ports, Port Proxy, Network Adapters and DNS Lookup pages, the SSH Config page
and the rest of the WSL page don't need administrator rights, and stopping a process only asks for
approval if Windows refuses to let you stop it as yourself.

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

### Ports

*Network → Ports*

Shows which application is using which port — what `netstat -ano` plus a trip to Task Manager's
Details tab would tell you, joined into one view. It needs no administrator rights.

- **List**: every TCP connection and UDP endpoint, sorted by local port, with its protocol, local
  address, remote address, state, and the owning process and PID. IPv6 addresses are shown in
  brackets (`[::1]:8080`).
- **Filters**: a state filter — **Listening** (the default; UDP endpoints are included, since
  they're bound and waiting for traffic), **Established**, or **All states** — plus a protocol
  filter (All / TCP / UDP).
- **Search**: matches port, address, PID, process name, executable path, command line, description,
  company, and Windows service name — handy for "who is using 8080?" or "what's this `svchost.exe`
  listening on?".
- **Process details**: click a row to expand it. You get the process name and PID, description,
  company and version (from the executable's file properties), start time, parent process, memory
  use, the Windows services it hosts (which tells you what's behind a shared `svchost.exe`), and
  the full executable path and command line.
- **Refresh**: connections change constantly, so the data is only cached for about ten seconds —
  use the refresh button for a fresh read.
- **Reserved port ranges**: an expandable section at the bottom lists the TCP and UDP port blocks
  Windows has reserved (`netsh int ipv4 show excludedportrange`) — Hyper-V, WSL and Docker are the
  usual owners. A port inside one of these can't be bound by an application even though nothing
  listens on it, which is the classic cause of "port already in use" with an empty list. If you
  search for a port number that isn't in use but falls inside a reserved range, the page says so.
  A `*` marks an administered exclusion (set explicitly rather than picked dynamically).
- **Free a port**: in an expanded row, **Stop process** force-closes the owning process after a
  confirmation. It first tries with your own rights and only asks for administrator approval (one
  UAC prompt) if Windows refuses. If the process hosts Windows services, each gets a **Stop
  service** button, which is cleaner than killing the process (a service manager may restart a
  killed process) and requires administrator approval. Critical Windows processes (such as
  `System`, `csrss.exe`, `lsass.exe`) are never offered, and the app checks the PID still belongs
  to the same process before stopping it, in case the list is out of date.

Windows hides the executable path and command line of processes owned by other users or the system
from non-administrator accounts, so those rows show "—" for those fields. If a process exits
between reading the list and clicking its row, the details say no information is available.

### Port Proxy

*Network → Port Proxy*

Manages `netsh interface portproxy` rules, which forward a port on this PC to another address —
the usual way to reach a service inside WSL or a container from elsewhere on the network. Windows
has no GUI for them. Viewing rules needs no administrator rights; adding or removing one does
(one UAC prompt, takes effect immediately).

- **List**: each rule's type (IPv4→IPv4, IPv4→IPv6, IPv6→IPv4 or IPv6→IPv6), the address and port
  it listens on, and where it forwards to.
- **Add a rule**: pick the type, then the listen address and port and the connect address (an IP
  address or hostname) and port. Ports must be 1–65535.
- **Remove**: deletes a rule after a confirmation.

A rule only forwards traffic — Windows Firewall still has to allow inbound connections on the
listen port.

### Network Adapters

*Network → Network Adapters*

One card per network adapter: status, IPv4 and IPv6 addresses, default gateway, DNS servers, whether
DHCP is on, MAC address, link speed, MTU, media type, and bytes received and sent (totals since the
adapter came up). Physical adapters are listed first; a checkbox hides virtual ones (WSL, Hyper-V,
VPN and similar). Viewing needs no administrator rights.

- **Enable / Disable**: turns the adapter on or off. Requires administrator rights.
- **Renew DHCP**: asks the DHCP server for a fresh lease (`ipconfig /renew`) — only available for
  adapters that use DHCP and are enabled. Requires administrator rights.

The data is cached for about fifteen seconds; use the refresh button for a fresh read.

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

### DNS Lookup

*Network → DNS Lookup*

A `dig`-style query tool. Enter a name, pick a record type (A, AAAA, CNAME, MX, NS, TXT, SOA, PTR,
SRV, CAA or DNSKEY) and, optionally, a specific DNS server to ask (an IP address) — leave it empty
to use the system resolver. Results show each record's name, type, TTL, section (Answer, Authority
or Additional) and data, plus the query time. The hosts file, LLMNR and mDNS are skipped, so the
answer is what DNS itself says — handy for comparing what two servers return, or checking that a
change has propagated. An unresolvable name shows the resolver's own error (for example "DNS name
does not exist"). No administrator rights are needed.

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
- **Reordering**: drag an entry by the grip handle on the left of its row and drop it on another
  row to move it above (dragging up) or below (dragging down) that row — drop on the first or last
  row to send an entry to the very top or bottom. Only the dragged line moves; comments and blank
  lines stay where they were. Requires administrator rights (one UAC prompt per move).

### SSH Config

*Network → SSH Config*

Manages the `Host` entries in `~/.ssh/config` (`C:\Users\<you>\.ssh\config`) — the aliases OpenSSH
expands before connecting, so `ssh prod` can stand in for a full user, address, port and key. The
file belongs to your account, so nothing on this page needs administrator rights.

- **List**: one row per `Host` block — alias, `user@hostname` (plus `via <proxy jump>` when set),
  identity file and port. A "+N other options" note counts options in the block this page doesn't
  manage (e.g. `ForwardAgent`).
- **Add / edit**: alias, hostname, user, port, identity file and proxy jump. Editing only touches
  those keys — every other line and comment in the block is kept, and clearing a field removes
  that key. Port must be 1–65535, and values can't contain newlines or quotes. An alias that
  already exists can't be added again.
- **Remove**: deletes the whole `Host` block, including options the page doesn't show.
- **Raw editor**: an expandable text editor for the whole file — use it for `Match` blocks,
  `Include` lines and anything else the list doesn't cover. Saving replaces the entire file.

If the file changes on disk between loading the list and clicking Edit or Remove, the action is
refused with a "refresh and try again" message rather than touching the wrong lines.

### WSL

*Network → WSL*

Windows Subsystem for Linux: see what's running, stop things, restart WSL, and edit the global
`.wslconfig`. Everything except the force restart runs without administrator rights.

- **Distributions**: each installed distro with its state (running/stopped), WSL version and a
  "Default" badge. Per row you can **stop** a distro (`wsl --terminate`) or **set it as default**.
- **Shut down WSL**: runs `wsl --shutdown` after a confirmation — stops every distro and the WSL
  virtual machine. WSL starts again the next time you use it, and that's also when a changed
  `.wslconfig` takes effect. Unsaved work inside running distros is lost.
- **`.wslconfig` form**: memory limit, processors, swap size, networking mode, auto memory reclaim,
  localhost forwarding and nested virtualization (the `[wsl2]` section of `%USERPROFILE%\.wslconfig`).
  Leaving a field on "Default" removes that key from the file; every other line, comment and
  section is kept. Changes apply after WSL is shut down and started again.
- **Raw editor**: an expandable text editor for the whole `.wslconfig`, for sections and options
  the form doesn't cover. Saving replaces the entire file.

#### When WSL is broken: Force restart

If WSL stops answering (for example the Docker Desktop integration is gone and `wsl --shutdown`
hangs), the page notices — status checks time out instead of freezing the app — and shows
"WSL isn't responding" with the **Force restart** button highlighted. Force restart:

1. Closes Docker Desktop first, if you leave **Also restart Docker Desktop** ticked.
2. Stops the WSL service (`WSLService` / `LxssManager`), kills any leftover WSL processes
   (`wsl`, `wslhost`, `wslrelay`, `wslservice`, `wslg`, `vmmem`, `vmmemWSL`) and starts the service
   again. This step needs administrator approval (one UAC prompt).
3. Starts WSL again — the distros that were running before, or the default distro if none could be
   detected — and then Docker Desktop. These are started **without** elevation, so nothing runs as
   administrator by accident.

Unsaved work inside running distros is lost, and Windows may refuse to kill `vmmem`; stopping the
service is what actually tears the VM down. If a distro or Docker Desktop doesn't come back, a
warning toast says which.

When Docker Desktop is installed in its default location (`Program Files\Docker\Docker`), a
separate **Restart Docker Desktop** button is also available: it closes Docker Desktop and its
helper processes, then launches it again — for when WSL is fine but Docker's integration is
missing or stuck. It needs no administrator rights, and running containers are stopped.

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
"Check for updates" — click it to check on demand (you'll get a toast either way: "You're up to
date (vX.Y.Z)", naming the version you're running, or "Couldn't check for updates" followed by the
actual reason). A release only counts once it's published on GitHub — drafts are ignored, and the
installers can take a few minutes to finish uploading. When a newer version exists, that same spot turns into a prominent button showing
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
