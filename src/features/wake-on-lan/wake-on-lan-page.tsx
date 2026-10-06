import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Pencil, Plus, Power, Trash2, Users } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAdapters } from "@/features/adapters/use-adapters";

interface WolDevice {
  id: string;
  name: string;
  mac: string;
  host: string | null;
  broadcast: string | null;
  port: number;
}

interface Neighbor {
  ipAddress: string;
  linkLayerAddress: string;
  state: string;
  interfaceAlias: string;
  family: string;
}

interface SendResult {
  sentOn: number;
  failures: string[];
}

interface PingResult {
  replies: { success: boolean }[];
}

type Phase = "sending" | "waiting" | "online" | "timeout" | "sent" | "error";
interface Status {
  phase: Phase;
  detail?: string;
}

const AUTO = "__auto__";
const POLL_EVERY_MS = 5000;
const POLL_FOR_MS = 90_000;

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function stripPrefix(address: string) {
  return address.split("/")[0];
}

function DeviceDialog({
  device,
  open,
  onClose,
  onSaved,
  t,
}: {
  /** `null` when adding. */
  device: WolDevice | null;
  open: boolean;
  onClose: () => void;
  onSaved: (devices: WolDevice[]) => void;
  t: TFunction;
}) {
  const [name, setName] = useState("");
  const [mac, setMac] = useState("");
  const [host, setHost] = useState("");
  const [broadcast, setBroadcast] = useState("");
  const [port, setPort] = useState("9");
  const [neighbors, setNeighbors] = useState<Neighbor[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(device?.name ?? "");
    setMac(device?.mac ?? "");
    setHost(device?.host ?? "");
    setBroadcast(device?.broadcast ?? "");
    setPort(String(device?.port ?? 9));
    setError(null);
    // Devices this PC has recently seen on the network, to copy a MAC from.
    invoke<Neighbor[]>("get_neighbors")
      .then((all) =>
        setNeighbors(
          all.filter(
            (n) =>
              n.family === "IPv4" &&
              n.state !== "Permanent" &&
              n.state !== "Incomplete" &&
              n.linkLayerAddress &&
              !/^(00-00-00-00-00-00|FF-FF-FF-FF-FF-FF)$/i.test(n.linkLayerAddress),
          ),
        ),
      )
      .catch(() => setNeighbors([]));
    // Keyed on the device id so a background refresh doesn't reset the form.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, device?.id]);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      const devices = await invoke<WolDevice[]>("save_wol_device", {
        device: {
          id: device?.id ?? "",
          name,
          mac,
          host: host.trim() || null,
          broadcast: broadcast.trim() || null,
          port: Number(port) || 0,
        },
      });
      onSaved(devices);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !saving && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{device ? t("wol.edit.title") : t("wol.add.title")}</DialogTitle>
          <DialogDescription>{t("wol.dialogDescription")}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          {neighbors.length > 0 && (
            <div className="flex flex-col gap-1.5">
              <Label>{t("wol.fromNeighbors")}</Label>
              <Select
                value=""
                onValueChange={(value) => {
                  const n = neighbors.find((x) => x.ipAddress === value);
                  if (!n) return;
                  setMac(n.linkLayerAddress);
                  setHost(n.ipAddress);
                }}
              >
                <SelectTrigger size="sm" className="w-full">
                  <SelectValue placeholder={t("wol.pickNeighbor")} />
                </SelectTrigger>
                <SelectContent>
                  {neighbors.map((n) => (
                    <SelectItem key={n.ipAddress} value={n.ipAddress}>
                      {n.ipAddress} — {n.linkLayerAddress}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="wolName">{t("wol.nameLabel")}</Label>
              <Input id="wolName" value={name} onChange={(e) => setName(e.currentTarget.value)} placeholder="Office PC" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="wolMac">{t("wol.macLabel")}</Label>
              <Input
                id="wolMac"
                value={mac}
                onChange={(e) => setMac(e.currentTarget.value)}
                placeholder="AA-BB-CC-DD-EE-FF"
                className="font-mono"
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="wolHost">{t("wol.hostLabel")}</Label>
              <Input
                id="wolHost"
                value={host}
                onChange={(e) => setHost(e.currentTarget.value)}
                placeholder="192.168.1.20"
              />
              <p className="text-xs text-muted-foreground">{t("wol.hostHint")}</p>
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="wolPort">{t("wol.portLabel")}</Label>
              <Input id="wolPort" value={port} onChange={(e) => setPort(e.currentTarget.value)} inputMode="numeric" />
            </div>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="wolBroadcast">{t("wol.broadcastLabel")}</Label>
            <Input
              id="wolBroadcast"
              value={broadcast}
              onChange={(e) => setBroadcast(e.currentTarget.value)}
              placeholder="255.255.255.255"
            />
            <p className="text-xs text-muted-foreground">{t("wol.broadcastHint")}</p>
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={saving}>
            {t("wol.cancel")}
          </Button>
          <Button onClick={save} disabled={saving || !name.trim() || !mac.trim()}>
            {saving && <Loader2 className="animate-spin" />}
            {t("wol.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function WakeOnLanPage() {
  const { t } = useTranslation();
  const { adapters } = useAdapters();

  const [devices, setDevices] = useState<WolDevice[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [editing, setEditing] = useState<WolDevice | null>(null);
  const [adding, setAdding] = useState(false);
  const [deleting, setDeleting] = useState<WolDevice | null>(null);
  const [sendFrom, setSendFrom] = useState(AUTO);
  const [status, setStatus] = useState<Record<string, Status>>({});
  // Stops polling for a device that was woken again, or when leaving the page.
  const runs = useRef<Record<string, number>>({});
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    invoke<WolDevice[]>("get_wol_devices")
      .then(setDevices)
      .catch(() => setDevices([]))
      .finally(() => setLoaded(true));
  }, []);

  // Adapters that are up and have an IPv4 address; physical ones first.
  const usable = useMemo(
    () =>
      adapters
        .filter((a) => a.status === "Up" && a.ipv4.length > 0)
        .map((a) => ({ name: a.name, ip: stripPrefix(a.ipv4[0]), virtual: a.isVirtual })),
    [adapters],
  );

  const localIps = useCallback((): string[] => {
    if (sendFrom !== AUTO) return [sendFrom];
    // "Automatic": every physical adapter that's up, so the packet reaches
    // the real LAN instead of leaving through a WSL/VPN adapter.
    return usable.filter((a) => !a.virtual).map((a) => a.ip);
  }, [sendFrom, usable]);

  const setPhase = (id: string, next: Status) => setStatus((prev) => ({ ...prev, [id]: next }));

  async function wake(device: WolDevice) {
    const run = (runs.current[device.id] ?? 0) + 1;
    runs.current[device.id] = run;
    setPhase(device.id, { phase: "sending" });
    try {
      const result = await invoke<SendResult>("send_wol", {
        mac: device.mac,
        broadcast: device.broadcast,
        port: device.port,
        localIps: localIps(),
      });
      if (!device.host) {
        setPhase(device.id, { phase: "sent", detail: t("wol.sentOn", { count: result.sentOn }) });
        return;
      }

      // No reply exists for a magic packet; the only way to know it worked
      // is to see the machine start answering.
      const started = Date.now();
      setPhase(device.id, { phase: "waiting" });
      while (mounted.current && runs.current[device.id] === run && Date.now() - started < POLL_FOR_MS) {
        await new Promise((r) => setTimeout(r, POLL_EVERY_MS));
        if (!mounted.current || runs.current[device.id] !== run) return;
        try {
          const ping = await invoke<PingResult>("ping_host", { target: device.host });
          if (ping.replies.some((r) => r.success)) {
            setPhase(device.id, {
              phase: "online",
              detail: t("wol.onlineAfter", { seconds: Math.round((Date.now() - started) / 1000) }),
            });
            return;
          }
        } catch {
          // keep waiting; the host may not resolve until it's up
        }
      }
      if (mounted.current && runs.current[device.id] === run) {
        setPhase(device.id, { phase: "timeout", detail: t("wol.timeoutAfter", { seconds: POLL_FOR_MS / 1000 }) });
      }
    } catch (err) {
      setPhase(device.id, { phase: "error", detail: errorMessage(err) });
      toast.error(t("wol.sendError", { error: errorMessage(err) }));
    }
  }

  async function confirmDelete() {
    const device = deleting;
    if (!device) return;
    setDeleting(null);
    try {
      setDevices(await invoke<WolDevice[]>("delete_wol_device", { id: device.id }));
      toast.success(t("wol.deleteSuccess", { name: device.name }));
    } catch (err) {
      toast.error(errorMessage(err));
    }
  }

  const statusLine = (id: string) => {
    const s = status[id];
    if (!s) return null;
    switch (s.phase) {
      case "sending":
        return <span className="flex items-center gap-1 text-muted-foreground"><Loader2 className="size-3 animate-spin" />{t("wol.sending")}</span>;
      case "waiting":
        return <span className="flex items-center gap-1 text-muted-foreground"><Loader2 className="size-3 animate-spin" />{t("wol.waiting")}</span>;
      case "online":
        return <span className="text-green-600 dark:text-green-400">{s.detail}</span>;
      case "timeout":
        return <span className="text-amber-600 dark:text-amber-400">{s.detail}</span>;
      case "sent":
        return <span className="text-muted-foreground">{s.detail}</span>;
      default:
        return <span className="text-destructive">{s.detail}</span>;
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("wol.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("wol.subtitle")}</p>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Button onClick={() => setAdding(true)}>
          <Plus />
          {t("wol.add.button")}
        </Button>
        <div className="flex items-center gap-2">
          <Label className="text-sm">{t("wol.sendFrom")}</Label>
          <Select value={sendFrom} onValueChange={(v) => setSendFrom(v as string)}>
            <SelectTrigger size="sm" className="w-64">
              <SelectValue>
                {(v: string) =>
                  v === AUTO ? t("wol.automatic") : `${usable.find((a) => a.ip === v)?.name ?? v} (${v})`
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={AUTO}>{t("wol.automatic")}</SelectItem>
              {usable.map((a) => (
                <SelectItem key={a.ip} value={a.ip}>
                  {a.name} ({a.ip})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {loaded && devices.length === 0 && (
        <div className="rounded-lg border p-4 text-sm text-muted-foreground">
          <p>{t("wol.none")}</p>
          <p className="mt-2">{t("wol.noneHint")}</p>
        </div>
      )}

      {devices.length > 0 && (
        <div className="rounded-lg border">
          {devices.map((d) => (
            <div key={d.id} className="flex flex-wrap items-center gap-3 border-b px-3 py-3 last:border-b-0">
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium">{d.name}</span>
                  <Badge variant="outline" className="font-mono text-xs">
                    {d.mac}
                  </Badge>
                  {d.host && <span className="text-xs text-muted-foreground">{d.host}</span>}
                </div>
                <div className="mt-0.5 min-h-4 text-xs">{statusLine(d.id)}</div>
              </div>
              <Button
                size="sm"
                onClick={() => wake(d)}
                disabled={status[d.id]?.phase === "sending" || status[d.id]?.phase === "waiting"}
              >
                <Power />
                {t("wol.wake")}
              </Button>
              <Button variant="ghost" size="icon-sm" aria-label={t("wol.edit.button")} onClick={() => setEditing(d)}>
                <Pencil />
              </Button>
              <Button variant="ghost" size="icon-sm" aria-label={t("wol.delete.button")} onClick={() => setDeleting(d)}>
                <Trash2 />
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className="flex gap-3 rounded-lg border bg-muted/30 p-3 text-sm text-muted-foreground">
        <Users className="mt-0.5 size-4 shrink-0" />
        <p>{t("wol.requirements")}</p>
      </div>

      <DeviceDialog
        device={editing}
        open={adding || editing !== null}
        onClose={() => {
          setAdding(false);
          setEditing(null);
        }}
        onSaved={setDevices}
        t={t}
      />

      <Dialog open={deleting !== null} onOpenChange={(o) => !o && setDeleting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("wol.delete.title", { name: deleting?.name ?? "" })}</DialogTitle>
            <DialogDescription>{t("wol.delete.description")}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleting(null)}>
              {t("wol.cancel")}
            </Button>
            <Button variant="destructive" onClick={confirmDelete}>
              {t("wol.delete.confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
