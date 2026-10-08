# zagzig-tools User Manual

*A Windows network and admin toolkit, with a terminal companion.*

This manual covers both apps in the project:

- **zagzig-tools** is the desktop app (Windows only), with network and admin tools.
- **zagzig-tui** is a terminal app with a subset of the diagnostics (Windows and Linux).

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

Both apps are published as GitHub Releases at `github.com/VladRafli/zagzig-tools/releases`.

### Desktop app (Windows)

Download and run one of:

- `zagzig-tools_<version>_x64_en-US.msi`, the standard Windows Installer package
- `zagzig-tools_<version>_x64-setup.exe`, the NSIS installer

Either one adds a Start Menu shortcut. Installing and daily use need no administrator rights. See
[Administrator rights](#administrator-rights) for when they are needed.

### Terminal UI (Windows or Linux)

**One command:** the installer downloads the latest release, checks its SHA-256 and puts the program
on your PATH. No administrator rights or root needed.

```powershell
irm https://raw.githubusercontent.com/VladRafli/zagzig-tools/main/install.ps1 | iex
```

```sh
curl -fsSL https://raw.githubusercontent.com/VladRafli/zagzig-tools/main/install.sh | sh
```

The first is for Windows (PowerShell), the second for Linux. Set `ZAGZIG_VERSION` (like `0.14.0`)
first to install a specific version, or `ZAGZIG_INSTALL_DIR` to choose the folder. Windows installs to
`%LOCALAPPDATA%\Programs\zagzig-tui` and Linux to `~/.local/bin`. Read a script before piping it
into a shell if you prefer, since both are short.

**By hand:** download and unzip the one that matches your machine:

- `zagzig-tui-x86_64-pc-windows-msvc.zip` gives `zagzig-tui.exe`
- `zagzig-tui-x86_64-unknown-linux-gnu.zip` gives `zagzig-tui`

Then run the binary from a terminal (`.\zagzig-tui.exe` on Windows, `./zagzig-tui` on Linux, after
`chmod +x zagzig-tui` if needed).

### Verifying your download

Each release includes `checksums.txt` with the SHA-256 of every other file:

- **Windows (PowerShell):** `Get-FileHash -Algorithm SHA256 <file>`, then compare the hash by eye
- **Linux, macOS, WSL:** `sha256sum -c checksums.txt`, run from the download folder

This only checks that the download is complete. It isn't a signature. See
[Staying up to date](#5-staying-up-to-date) for how updates are verified.

---

## 2. Getting started with the desktop app

The app opens on the **Dashboard**, a grid of icons, one per feature, grouped as **Overview**,
**Network**, **System** and **Dev Tools**. Click an icon to open that page. The sidebar is hidden on
the Dashboard and opens when you pick a feature. The **Dashboard** button in the top bar goes back,
and the button at its left toggles the sidebar (`Ctrl+B` too). The top bar also shows your
administrator status. The sidebar footer has the update control, theme switcher and language
switcher.

**Quick search:** press `Ctrl+K` (or click **Search** in the top bar) from any page, type part of a
feature name and press `Enter` to open it. `↑`/`↓` move through the results and `Esc` closes it.
Letters in order also match, so `dnsm` finds DNS Monitor. With nothing typed, starred features come
first.

This manual is built into the app under **User Manual** (searchable, English only).

### Administrator rights

These actions need administrator approval:

- **Network:** removing an NRPT rule, adding or removing a route, changing DNS servers or the WinHTTP
  proxy, adding or removing a port proxy rule, enabling, disabling or renewing an adapter, clearing
  ARP entries, showing a saved Wi-Fi password
- **Files:** editing, reordering or restoring the hosts file
- **Firewall:** creating, deleting or switching a rule
- **System:** starting, stopping or reconfiguring a service, changing a System environment variable,
  switching an all-users startup entry, force restarting WSL
- **Certificates:** importing into or deleting from a machine-wide (`Local Machine`) store

Everything else works without them. That includes reading any page, the SSH Config page, most of
WSL, connecting a VPN, importing a certificate into your own account, editing your own (User)
environment variables, and flushing the DNS cache. Stopping a process asks for approval only if
Windows refuses to let you do it yourself.

The app itself **never runs elevated**. Each privileged action shows one UAC prompt for that change.
Locked controls show a lock icon, and a tooltip says why.

If your account isn't an administrator, those controls stay locked and the top bar says "Standard
user". If your account could elevate but the app isn't running as administrator, the Dashboard shows
a **"Running as a standard user"** banner with a **Restart as administrator** button.

### Theme and language

The sidebar footer has a theme selector (System, Light, Dark) and a language selector. English and
Indonesian are built in. Other languages can be added without a new release.

#### Adding another language

Open **Languages** under **Overview**. A language is one JSON file with the same structure as the
English one.

1. Under **Add your own language**, choose **Start from English** (or **Start from Indonesian**) and
   click **Export template…**.
2. In the file's `$meta` section, set `code` (like `fr` or `pt-br`) and `name` (written in that
   language). Add `"dir": "rtl"` for a right-to-left language.
3. Translate the values. Keep the keys, `{{placeholders}}` and tags like `<0>…</0>` as they are.
   Save as `<code>.json`, for example `fr.json`.
4. Click **Add language…** and pick the file. It shows up in the list and in the language selector.

Good to know:

- **Partial translations work.** Missing strings show in English.
- **Safety checks.** A string whose placeholders don't match English, or a key the app doesn't have,
  is skipped. The Languages page shows coverage and skipped strings.
- **Plural forms.** Add forms like `_few` or `_many` next to `_one` and `_other` if the language needs
  them.
- **Where they live.** In `%APPDATA%\com.vladrafli.zagzig-tools\languages`. **Open languages folder**
  goes there, **Reload languages** picks up files you copied in, and a custom language can be removed
  from its row. English and Indonesian can't be replaced or removed.
- **Updates.** New app versions add strings that show in English until you translate them. Export a
  fresh template to see what's new.

---

## 3. Desktop app features

### Dashboard

The landing page. It shows your signed-in user (on a domain-joined PC, also title, department and
manager from LDAP) and the administrator banner when it applies. Below is a small icon for every
feature, grouped like the sidebar. Click one to open it.

**Starred:** hover an icon and click its star to keep that feature in the **Starred** row at the top
of the Dashboard and in a **Starred** group at the top of the sidebar. The sidebar also shows a star on
hover for each item. Click a filled star to remove it. Stars are stored on this PC only.

### NRPT Rules

*Network → NRPT Rules*

The Name Resolution Policy Table, the same rules `Get-DnsClientNrptRule` reports. They send DNS
queries for a namespace (like `.corp.example.com`) to specific servers. Expand a rule to see every
field, like DNSSEC, DirectAccess and IPsec settings.

- **Remove** is immediate, with one UAC prompt if needed.
- **New rule** adds the rule to Windows (one UAC prompt, immediate). Enter a namespace and at least
  one DNS server (IP address or host name), and optionally a comment. **Details** holds the name
  encoding, DNSSEC options (turn on DNSSEC first), IPsec encryption levels (None, Low, Medium, High),
  the IPsec CA restriction and DirectAccess settings. A DirectAccess proxy is written as
  `name:port`. Windows rejects some combinations and its message is shown in the form.

### Connection Test

*Network → Connection Test*

Enter a hostname or IP and click **Run test**:

- **Is it reachable?** 4 pings, with the average reply time and how many were answered.
- **Path it took.** A traceroute hop list. Each hop shows its reverse DNS name like `name [ip]` when
  one exists. Many hops have no name, which is normal.

The last tests are kept in **History**, where you can re-run or clear them.

**Saved hosts:** **Save this host** stores the current host under a name, and the dropdown loads one
back. The list is shared with the **Port Scanner** (which also remembers ports) and the **TLS
Inspector** (which also remembers the port). Saving under an existing name updates it. **Manage**
removes entries. They are stored on this PC only.

### Ports

*Network → Ports*

Shows which application uses which port, like `netstat -ano` joined with Task Manager's Details tab.
No administrator rights needed.

- **List:** every TCP connection and UDP endpoint by local port, with protocol, addresses, state and
  the owning process and PID. IPv6 addresses are in brackets (`[::1]:8080`).
- **Filters:** state (**Listening** by default, **Established**, **All states**) and protocol (All,
  TCP, UDP). UDP endpoints count as listening.
- **Search:** matches port, address, PID, process name, path, command line, description, company and
  service name. Good for "who is using 8080?".
- **Process details:** click a row for the process name, PID, description, company, version, start
  time, parent, memory, hosted Windows services (what's behind a shared `svchost.exe`), full path and
  command line.
- **Refresh:** data is cached for about ten seconds, so use the refresh button for a fresh read.
- **Reserved port ranges:** the section at the bottom lists port blocks Windows reserved (Hyper-V,
  WSL and Docker are the usual owners). An application can't bind a port inside one, even though
  nothing listens on it. That is the classic "port already in use" with an empty list. `*` marks an
  administered exclusion.
- **Free a port:** **Stop process** force-closes the owner after confirmation. It tries with your own
  rights first and asks for approval only if Windows refuses. For hosted services, **Stop service** is
  cleaner (a killed process may be restarted) and needs approval. Critical processes (`System`,
  `csrss.exe`, `lsass.exe`) are never offered, and the PID is rechecked before stopping.
- **Allow in firewall:** on a listening port, **Allow port N in firewall…** opens the Firewall rule
  dialog with the port, protocol and program filled in. Needs administrator rights.

Windows hides the path and command line of other users' processes from standard accounts, so those
show "—". If a process exits before you click its row, the details say no information is available.

### Port Proxy

*Network → Port Proxy*

Manages `netsh interface portproxy` rules, which forward a port on this PC to another address. This
is the usual way to reach a service in WSL or a container from the network. Viewing needs no
administrator rights. Adding or removing does (one UAC prompt, immediate).

- **List:** each rule's type (IPv4 or IPv6 to IPv4 or IPv6), listen address and port, and target.
- **Add a rule:** pick the type, the listen address and port, and the connect address (IP or
  hostname) and port. Ports are 1 to 65535.
- **Remove:** deletes a rule after confirmation.

A rule only forwards. Windows Firewall must still allow inbound connections on the listen port.

### Port Scanner

*Network → Port Scanner*

Checks which TCP ports on one host accept a connection. Scan only hosts you own or may test. No
administrator rights needed.

- **Host and ports:** a hostname or IP and a preset (**Common ports**, **Web servers**,
  **Databases**, **Dev servers**, **Well-known (1 to 1024)**) or **Custom…** like `22,80,443,8000-8100`.
  One host at a time, up to 4,096 ports.
- **Timeout per port:** 300 ms to 4 s. Longer is slower but surer on a slow network.
- **Results:** open ports with the service they usually carry, and counts of closed, filtered and
  failed ports. **Cancel** keeps what was found so far.

A port that answers is **open**, one that refuses is **closed**, and one that never answers is
probably **filtered** by a firewall. Windows can take a couple of seconds to refuse a connection to
this PC's own address, so a closed local port may count as filtered. Open ports are always right. It
can't scan UDP or sweep several hosts.

### Wake-on-LAN

*Network → Wake-on-LAN*

Turns on a PC that is off or asleep by sending a "magic packet". **The other PC needs no software**,
because its network card recognises the packet. No administrator rights needed.

- **Devices:** add a name and MAC address (`AA-BB-CC-DD-EE-FF`, colons and dots also work).
  **Fill in from a device this PC has seen** copies one from the Neighbors list. Devices are saved
  and can be edited or deleted.
- **Wake:** sends three copies from every physical adapter that is up, so the packet leaves through
  your real network and not WSL or VPN. **Send from** picks one adapter. A device can have its own UDP
  port (default 9) and broadcast address (default `255.255.255.255`, or your subnet's like
  `192.168.1.255`).
- **Did it work?** A magic packet gets no reply. With an **address to ping**, the app pings for up to
  90 seconds and shows "Online after N seconds". Without one, it only says the packet was sent.

It works only if Wake-on-LAN is on in the target's BIOS/UEFI and in its network card driver (Device
Manager, Power Management). Turning off Fast Startup helps from a full shutdown. The PC must be
plugged in, wired, and on the same network, since broadcasts don't cross routers. Wi-Fi PCs rarely
support it.

### Network Adapters

*Network → Network Adapters*

One card per adapter: status, IPv4 and IPv6 addresses, gateway, DNS servers, DHCP, MAC, link speed,
MTU, media type, and bytes received and sent since it came up. Physical adapters come first, and a
checkbox hides virtual ones (WSL, Hyper-V, VPN). Viewing needs no administrator rights. Data is cached
for about fifteen seconds.

- **Enable / Disable:** turns the adapter on or off. Needs administrator rights.
- **Renew DHCP:** asks for a fresh lease (`ipconfig /renew`), only for enabled DHCP adapters. Needs
  administrator rights.

### Network Routes

*Network → Network Routes*

The IP routing table, like `route print`, `route add` and `route delete`. Local and system routes are
hidden unless you turn on **Show system routes**.

- **Add route:** destination (CIDR), next hop, interface, optional metric, and whether to keep it
  across restarts. Needs administrator rights, immediate.
- **Remove route:** the delete button on a row.

### DNS Servers

*Network → DNS Servers*

DNS servers per adapter, as many as you need (the Windows dialog offers only two).

- **Edit:** add, remove or reorder servers. Order is the order they are tried. Needs administrator
  rights.
- **Reset to automatic:** back to DHCP-provided servers.

### DNS Lookup

*Network → DNS Lookup*

A `dig`-style query tool. Enter a name, pick a record type (A, AAAA, CNAME, MX, NS, TXT, SOA, PTR, SRV,
CAA, DNSKEY) and optionally a DNS server IP (empty uses the system resolver). Results show name, type,
TTL, section and data, plus the query time. The hosts file, LLMNR and mDNS are skipped, so you see what
DNS itself says. Good for comparing two servers or checking propagation. No administrator rights
needed.

### DNS Cache

*Network → DNS Cache*

The resolver cache, like `ipconfig /displaydns`. It lists every cached record (name, type, data,
remaining TTL), including negative entries.

- **Flush DNS cache:** clears it all. This is the one write that needs **no** administrator rights.

### DNS Monitor

*Network → DNS Monitor*

A background watcher. Add a hostname, optionally a DNS server (blank is the system default), and an
interval (1 second to 30 minutes). It keeps resolving the name and logs success, time and addresses,
even while you are on another page. Each monitor starts, stops and clears its log on its own.

### Hosts File

*Network → Hosts File*

Edits `C:\Windows\System32\drivers\etc\hosts`. All changes need administrator rights and are
immediate.

- **Structured view:** a table of entries (enabled toggle, IP, hostnames, comment) with per-row
  enable, disable and delete, plus an add form.
- **Raw editor:** the whole file as text, for anything the table doesn't understand. Saving replaces
  the file.
- **Reordering:** drag an entry by its grip and drop it on another row to move it above (dragging up)
  or below (dragging down). Drop on the first or last row for the very top or bottom. Comments and
  blank lines stay put. One UAC prompt per move.
- **Backups:** a copy is saved before every change (add, remove, switch, reorder, raw edit, restore)
  unless it matches the newest one. The newest 30 are kept in the app's data folder. **Back up now**
  makes one on demand. **Compare / restore** shows the lines a restore would add and remove before you
  confirm. A restore backs up the current file first, so it can be undone. Backups include comments.

### SSH Config

*Network → SSH Config*

Manages `Host` entries in `~/.ssh/config` (`C:\Users\<you>\.ssh\config`), the aliases OpenSSH expands
so `ssh prod` can stand for a full user, address, port and key. No administrator rights needed.

- **List:** one row per `Host` block with alias, `user@hostname` (plus `via <proxy jump>`), identity
  file and port. "+N other options" counts options the page doesn't manage, like `ForwardAgent`.
- **Add / edit:** alias, hostname, user, port, identity file and proxy jump. Editing touches only
  those keys, and clearing a field removes its key. Ports are 1 to 65535, values can't hold newlines
  or quotes, and an alias can't be added twice.
- **Remove:** deletes the whole `Host` block.
- **Raw editor:** the whole file, for `Match` blocks, `Include` lines and the rest.

If the file changes on disk after loading, Edit and Remove refuse and ask you to refresh.

### WSL

*Network → WSL*

Windows Subsystem for Linux: see what's running, stop things, restart WSL and edit `.wslconfig`.
Everything except Force restart works without administrator rights.

- **Distributions:** each distro's state, WSL version and a "Default" badge. **Stop** a distro
  (`wsl --terminate`) or **set it as default**.
- **Shut down WSL:** `wsl --shutdown` after confirmation. It stops every distro and the WSL VM, and a
  changed `.wslconfig` takes effect on the next start. Unsaved work in running distros is lost.
- **`.wslconfig` form:** memory limit, processors, swap, networking mode, auto memory reclaim,
  localhost forwarding and nested virtualization (the `[wsl2]` section of
  `%USERPROFILE%\.wslconfig`). A field left on "Default" removes its key, and everything else in the
  file is kept.
- **Raw editor:** the whole `.wslconfig`, for options the form doesn't cover.

#### When WSL is broken: Force restart

If WSL stops answering (for example Docker's integration is gone and `wsl --shutdown` hangs), the page
shows "WSL isn't responding" and highlights **Force restart**. It does this:

1. Closes Docker Desktop, if **Also restart Docker Desktop** is ticked.
2. Stops the WSL service (`WSLService` or `LxssManager`), kills leftover WSL processes (`wsl`,
   `wslhost`, `wslrelay`, `wslservice`, `wslg`, `vmmem`, `vmmemWSL`) and starts the service again.
   This needs administrator approval (one UAC prompt).
3. Starts WSL again (the distros that were running, or the default one), then Docker Desktop. Both
   start **without** elevation.

Unsaved work in running distros is lost, and Windows may refuse to kill `vmmem` (stopping the service
is what tears down the VM). A warning toast says which one didn't come back.

If Docker Desktop is in its default folder (`Program Files\Docker\Docker`), **Restart Docker
Desktop** also appears. It closes Docker and its helpers and launches it again, for when WSL is fine
but Docker's integration is stuck. It needs no administrator rights and stops running containers.

### Firewall

*Network → Firewall*

Lists every Windows Defender Firewall rule, useful for "why is this port blocked?". Reading needs no
administrator rights.

- **Profiles:** whether Domain, Private and Public are on or off (display only).
- **Rules:** name (with program path or group below), direction, action, protocol, local port and
  profile. Search matches name, program, group, protocol, ports and profile. Filters cover direction,
  action and enabled state. 150 rules show at a time, so use **Show more**.
- **On or off:** the switch at the start of a row. Needs administrator rights. Rules are matched by
  exact name, so a wildcard can't touch other rules.
- **New rule:** an allow or block rule for a port (single, several, or a range like `8000-8100`),
  inbound or outbound, TCP or UDP, optionally for one program (full path), on the profiles you pick.
  **Local network only** (default) limits an allow rule to your subnet. **Any address** opens it to
  everyone who can reach the PC, and the dialog warns you. Needs administrator rights.
- **From the Ports page:** **Allow port N in firewall…** opens the dialog already filled in.
- **Delete:** rules made here are tagged as this app's and show a trash button. They are the only
  rules this app deletes. Windows' own rules can be switched off but never removed from here.

### Neighbors (ARP)

*Network → Neighbors (ARP)*

The IP to MAC cache for IPv4 (ARP) and IPv6, showing which devices this PC recently talked to, with
state (Reachable, Stale, ...) and interface. Permanent entries are hidden unless you tick **Show
permanent entries**. Data is cached for about fifteen seconds.

- **Remove an entry** or **Clear cache** forces fresh lookups, useful after a device changes its IP or
  MAC. Both need administrator rights. Permanent entries are never removed.

### VPN

*Network → VPN*

The VPN connections built into Windows (Settings → Network → VPN, for you and all users), with
status, server, tunnel type, authentication, split tunneling and whether credentials are saved.
**Connect** and **Disconnect** use `rasdial` and need no administrator rights. A connection that needs
unsaved credentials fails with Windows' own error.

VPN apps with their own client and adapter (WireGuard, OpenVPN, vendor clients) aren't Windows VPN
profiles and don't appear. VPN connections often tie to NRPT rules, so this pairs with NRPT Rules.

### Wi-Fi

*Network → Wi-Fi*

Wireless networks saved on this PC: name, security, connect mode and auto-switch. Listing needs no
administrator rights. Without a wireless adapter or WLAN service, the page says so. Only saved
networks show, not nearby ones.

**Show password** displays a saved passphrase, like Windows' "View Wi-Fi security key". It needs
administrator approval (one UAC prompt) and a confirmation, because anyone who can see your screen
sees the password. It is read only when you confirm, kept only while the window is open (**Copy**
works), and never stored, cached or logged. Open networks and certificate or 802.1X networks have no
passphrase.

### Proxy Settings

*Network → Proxy Settings*

The **WinHTTP** proxy (`netsh winhttp show proxy`), a machine-wide setting separate from Settings →
Network. Windows Update's service and many background agents and CLI tools honor only this one, which
is why a wrong setting can break some things.

- View the current proxy (direct access, or server plus bypass list).
- **Set proxy:** server and optional bypass list. Needs administrator rights.
- **Reset to direct access:** removes the proxy.
- **Import from system proxy:** copies the Settings → Network → Proxy values into WinHTTP, the quick
  fix when a tool ignores the proxy you set elsewhere.

### Services

*System → Services*

Windows services like `services.msc`: display name, internal name, description, state and startup
type. Reading needs no administrator rights, and every change does (one UAC prompt).

- **Search and filters:** search matches name, description, account and path. Filter by state
  (Running, Stopped) and startup type (Automatic, Manual, Disabled). 100 services show at a time.
- **Start, stop, restart:** buttons on each row. Stop and restart also affect dependent services and
  ask first.
- **Startup type:** Automatic, Automatic (delayed), Manual or Disabled, from the row's dropdown.
  Disabled asks first, since not even Windows can start it. Driver services (boot or system start)
  can't be changed here.

Stopping or disabling a core service can break networking and other features. Services are matched by
exact name.

### Event Log

*System → Event Log*

Recent Windows events, filtered to what this app covers. Reading needs no administrator rights (the
Security log isn't offered).

- **Source:** **Network & DNS** (TCP/IP, DHCP client, DNS client, network location awareness, RAS/VPN,
  WLAN), **WSL, Hyper-V & Docker**, or the plain **System** or **Application** log. Missing Docker or
  Hyper-V logs are skipped quietly.
- **Level:** errors only, errors and warnings, or everything. **Time window:** last hour to 30 days.
  **Maximum events:** 100, 200 or 500.
- **Filter by text:** matches message or provider, applied with Enter. It searches the 2,000 newest
  matching events per source.
- Click an event for its full message, log and ID. Changing a setting reloads the list.

### Environment Variables

*System → Environment Variables*

The variables Windows hands to every program, including `PATH`. It replaces the system dialog with a
list editor and undo for every change. Reading needs no administrator rights. Your own (User)
variables can be edited without them, while **System** variables need approval (one UAC prompt per
change).

- **Two scopes:** **User variables** and **System variables**. Search matches names and values.
  *Expandable* variables (with `%VARIABLE%` references) are marked.
- **Add, edit, delete:** a name can't change once it exists (add a new one and delete the old one).
  Adding an existing name is refused. The **Expandable value** box turns on by itself when a value
  has `%SOMETHING%`, otherwise it would be stored literally. Values are saved as stored, so
  `%SystemRoot%` stays `%SystemRoot%`.
- **Edit as list:** for `PATH`-style values (or any value with `;`), each entry gets a row. Move,
  edit, remove and add entries. For folder lists, each row says whether the folder exists
  (`%VARIABLE%` is expanded, network paths aren't probed) and flags duplicates. **Remove duplicates**
  and **Remove missing folders** clean up in one click, and nothing is saved until **Save**. A
  counter warns past 2,047 characters, where older programs ignore the rest. The limit is 16,000.
- **Protected variables:** `Path`, `PATHEXT`, `ComSpec`, `SystemRoot`, `windir`, `TEMP`, `TMP` and a
  few more can be edited but not deleted.
- **Change history:** each change is recorded with the value it replaced. **Undo** restores it (or
  removes the variable if it was new) and is recorded too, so an undo can be undone. The last 50 are
  kept in the app's data folder, including any sensitive old values.

Changes go to the registry and Windows is told, so Explorer and newly started programs see them.
**Programs already running, like terminals and IDEs, keep the old environment** until restarted.

### Startup

*System → Startup*

What starts when you sign in: the registry Run keys (yours, all users', and the 32-bit all-users one)
and the two Startup folders (yours and all users'). Each entry shows description and company, command
line and source.

- **Switch:** turns an entry on or off with the same flag Task Manager uses, so the two agree.
  Turning off doesn't delete, and turning on restores it exactly. All-users entries need
  administrator approval, yours don't.
- **File not found:** shown in red when the target no longer exists, usually after an uninstall.
  Store-app folders can't be checked and are never flagged.
- **Show the file in Explorer:** opens the folder with the program selected.
- Search matches name, command, company and description. The filter shows all, enabled or disabled.

Scheduled tasks and boot services aren't listed. Use Services for those.

### Diagnostic Report

*System → Diagnostic Report*

Collects what the app knows about the PC into one Markdown report for a support ticket or colleague.
Nothing is uploaded. The report stays on screen until you copy it or save it as `.md` or `.txt`. No
administrator rights needed, and it takes a few seconds.

- **Sections** (all on by default): **System** (Windows version, hardware, uptime, administrator
  status), **Network adapters**, **DNS client settings**, **NRPT rules**, **Routes** (defaults and
  total), **Proxy**, **Hosts file** (active entries only), **Listening ports** (with processes, up to
  120), **Firewall** (profile state and rule counts), **WSL and Docker**, **Recent network errors**
  (last 24 hours). A section that can't be read says so and doesn't stop the rest.
- **Hide the computer and user names and MAC addresses** (on by default): they become `<computer>`,
  `<user>` and `xx-xx-xx-xx-xx-xx` everywhere, including paths and event messages.
- **Hide the last part of IPv4 addresses** (off by default): `192.168.1.20` becomes `192.168.1.x`.
  Loopback (`127.x`) and `0.0.0.0` stay, IPv6 isn't masked, and version numbers are untouched.

The report never contains saved Wi-Fi passwords or environment variable values. Still read it before
sharing, since hostnames, folder names and process names can identify you or your organisation.

### Code Signing

*Dev Tools → Code Signing*

A wrapper around `signtool.exe` from the Windows SDK, which isn't part of Windows. The page looks for
it in common Windows Kits paths and on `PATH`, and you can locate a copy yourself. It needs no
administrator rights, only a found `signtool.exe`.

- **Sign a file:** pick a file, then a certificate from your personal store (`CurrentUser\My`) or a
  `.pfx`/`.p12` file with its password. Choose the digest (SHA256 or SHA1), an optional timestamp
  server and an optional description. signtool's output shows inline.
- **Verify a signature:** checks whether a file is signed and trusted.

### TLS Inspector

*Dev Tools → TLS Inspector*

Connects to a server, does the TLS handshake and shows what it presents. Good for finding out why a
tool or browser refuses a local or internal HTTPS service. No administrator rights needed, and nothing
is sent after the handshake.

- **Host, port and server name:** the port defaults to 443. **Server name** is the name sent in the
  handshake (SNI), and empty means the host. Fill it in when connecting by IP or when a server picks a
  certificate by name.
- **Verdicts:** whether **Windows trusts** the certificate, whether its **names cover** the name you
  used, and how long until it **expires**. TLS 1.1 or older is flagged.
- **Connection:** TLS version, cipher, key exchange, hash and handshake time.
- **Server certificate:** subject, issuer, validity, key type and size, signature algorithm, serial,
  thumbprint and every name it covers.
- **Certificate chain:** each certificate up to the root, with any problem found at that step.

"Trusted" means *Windows* trusts it, which is what most Windows tools, .NET apps and PowerShell use.
Firefox and tools with their own authority list (Node.js, Python, Java) may decide differently. A
self-signed or private-CA certificate shows as not trusted until its CA is added to **Trusted Root
Certification Authorities** (use **Import…** on the Certificate Store page). A server that omits its
intermediate certificates is a common cause of "works in the browser, fails in my tool".

Certificates are read even when invalid, so a successful connection doesn't mean the server is safe.

### Certificate Store

*Dev Tools → Certificate Store*

Browse installed certificates without `certmgr.msc`. Switch between Personal, Trusted Root
Certification Authorities, Intermediate Certification Authorities and Trusted Publishers, for the
current user or the local machine.

- **View details:** subject, issuer, thumbprint, serial, friendly name, validity, private key and
  usage.
- **Export:** saves the public certificate (`.cer`).
- **Delete:** needs administrator rights only for **Local Machine** stores.
- **Import…:** adds certificates from `.cer`, `.crt`, `.der`, `.pem`, `.p7b` or a password-protected
  `.pfx`/`.p12`. It works in two steps so nothing is trusted blind. First the file is read and every
  certificate is listed (subject, issuer, expiry, thumbprint, and whether it is a CA, self-signed or
  has a private key). Only after you pick a store and confirm is anything added. PEM bundles import
  every certificate. A protected file asks for its password first, which is used once and not stored.

  Importing into **Trusted Root Certification Authorities** makes Windows trust that authority for
  everything, for your account or for all users in the Local Machine store. The dialog says so in
  red, so check the thumbprint against the one you were given. Windows also shows its own
  confirmation for a root certificate in your account. Any **Local Machine** store needs
  administrator rights.

---

## 4. Using the terminal UI (zagzig-tui)

Run `.\zagzig-tui.exe` (Windows) or `./zagzig-tui` (Linux). It opens a full-screen menu on the left,
the selected screen on the right, and a status bar with the keys that apply.

### Global keys (menu focused)

| Key | Action |
| --- | --- |
| `↑`/`↓` or `j`/`k` | Move between menu items |
| `Enter`, `Tab`, `→`, or `l` | Open the selected section |
| `Esc` (inside a section) | Back to the menu |
| `q` or `Esc` (in the menu) | Quit |
| `u` | Install an available update, or retry after an error |
| `r` | Restart after an update has installed |

### Screens

- **Dashboard:** running DNS monitors, DNS server groups last read, and your last connection test.
- **Connection Test:** type a host and press Enter to ping it 4 times. Results and a short history
  show inline.
- **DNS Servers:** a read-only view per adapter. Windows parses `ipconfig /all`. Linux uses
  `resolvectl status` when available, else `/etc/resolv.conf`, which on systemd-resolved only points
  at a local stub. Press `r` to refresh.
- **DNS Monitor:** `Tab`/`Shift+Tab` moves between hostname, server, interval and the monitor list.
  `←`/`→` change the interval. `Enter` adds a monitor from the form, or starts or stops the selected
  one. `x` removes it and `c` clears its log.
- **Ports:** what listens on which port, with the owning process and PID. Windows reads `netstat`
  and `tasklist`, Linux reads `ss` (run as root to see other users' processes).
- **Hosts File:** the active entries of the hosts file, read only (comments are skipped). Edit it in
  the desktop app.

Ports and Hosts File share the same keys: `↑`/`↓` or `j`/`k` scroll, `PgUp`/`PgDn` jump,
`/` filters (type, then `Enter`), `c` clears the filter and `r` refreshes.

### Linux-specific notes

Raw ICMP pings need elevated permissions on Linux. If Connection Test reports permission denied, run
as root, grant the binary `CAP_NET_RAW`, or allow unprivileged ping with
`sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"`.

---

## 5. Staying up to date

Both apps check the latest GitHub release at startup, every hour in the background, and on demand.
Only published releases count, since drafts are ignored and installers can take a few minutes to
upload.

**Desktop app:** the sidebar footer has an update control. Usually it reads "Check for updates".
Click it to check, and a toast says "You're up to date (vX.Y.Z)" or "Couldn't check for updates" with
the reason. When a newer version exists, it becomes a button with the version number. Click it to see
the release notes and **Install and restart**. The download is verified against a signing key built
into the app (Tauri's updater, cryptographically signed, not just HTTPS), installed, and the app
restarts. Background checks stay silent unless they find an update.

**Terminal UI:** press `u` in the menu to check. When an update exists, the status bar shows
`update available: vX.Y.Z   u: install and restart`. Press `u` to download and verify it (signed with
a separate Ed25519 key via [zipsign](https://github.com/Kijewski/zipsign), so an unsigned download is
rejected). Once installed, the bar shows `updated to vX.Y.Z, r: restart now`, and `r` relaunches.

If a check or install fails, the status bar shows the error with a `u: retry` hint.

---

## 6. Troubleshooting

**A button is locked with a padlock.** That action needs administrator rights your session doesn't
have. Hover it for the reason, or see [Administrator rights](#administrator-rights).

**"Not found" for signtool.exe.** It comes with the Windows SDK or Visual Studio Build Tools. Install
one, or use "Locate signtool.exe" to point at a copy.

**A DNS Monitor entry keeps failing.** Check the server field. A server you named may be unreachable
or not serve that record. Leave it blank to use the system resolver.

**The TUI reports a permission error on Connection Test (Linux).** See
[Linux-specific notes](#linux-specific-notes).

**An update fails to verify or install.** Retry with `u` (TUI) or reopen the update dialog (desktop).
A network hiccup during download is the usual cause. If it persists, download the release from
GitHub and check it against `checksums.txt`.

---

## 7. Getting help

- Issues and questions: `github.com/VladRafli/zagzig-tools/issues`
- Project overview and technical details: [`README.md`](../README.md) in the repository root
- License: MIT, see [`LICENSE`](../LICENSE)
