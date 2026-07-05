import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Calendar, Send, Youtube, Facebook, Instagram } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

function TikTokIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M19.59 6.69a4.83 4.83 0 0 1-3.77-4.25V2h-3.45v13.67a2.89 2.89 0 0 1-5.2 1.74 2.89 2.89 0 0 1 2.31-4.64 2.93 2.93 0 0 1 .88.13V9.4a6.84 6.84 0 0 0-1-.05A6.33 6.33 0 0 0 5 20.1a6.34 6.34 0 0 0 10.86-4.43v-7a8.16 8.16 0 0 0 4.77 1.52v-3.4a4.85 4.85 0 0 1-1-.1z" />
    </svg>
  );
}

const PLATFORMS = [
  { id: "YT", label: "YouTube", Icon: Youtube, color: "text-red-500" },
  { id: "IG", label: "Instagram", Icon: Instagram, color: "text-pink-500" },
  { id: "FB", label: "Facebook", Icon: Facebook, color: "text-blue-500" },
  { id: "TikTok", label: "TikTok", Icon: TikTokIcon, color: "text-cyan-500" },
];

export default function PublishConsolePage() {
  const { pieceId } = useParams<{ pieceId: string }>();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [captions, setCaptions] = useState<Record<string, string>>({});
  const [tags, setTags] = useState<Record<string, string>>({});
  const [scheduledAt, setScheduledAt] = useState("");
  const [publishing, setPublishing] = useState(false);
  const [status, setStatus] = useState<Record<string, string>>({});
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle(`Publish · ${pieceId ?? "new"}`);
    setSubTitle("Choose platforms, write captions, and schedule or publish now.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle, pieceId]);

  const togglePlatform = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const setPlatformStatus = (platforms: string[], message: string) => {
    setStatus((prev) => {
      const next = { ...prev };
      platforms.forEach((p) => (next[p] = message));
      return next;
    });
  };

  const publish = async (now: boolean) => {
    if (!pieceId || selected.size === 0) return;
    const platforms = Array.from(selected);
    setPublishing(true);
    setPlatformStatus(platforms, now ? "Publishing…" : "Scheduling…");
    try {
      if (now) {
        const res = await api.marketing.publishNow(pieceId, platforms);
        setPlatformStatus(platforms, res.urls ? "Published" : "Published");
        if (res.urls) {
          setStatus((prev) => {
            const next = { ...prev };
            Object.entries(res.urls!).forEach(([platform, url]) => {
              next[platform] = `Published · ${url}`;
            });
            return next;
          });
        }
        showToast("Published now", "success");
      } else {
        await api.marketing.schedulePublish(pieceId, platforms, scheduledAt);
        setPlatformStatus(platforms, `Scheduled for ${new Date(scheduledAt).toLocaleString()}`);
        showToast("Scheduled", "success");
      }
    } catch (err) {
      setPlatformStatus(platforms, "Failed");
      showToast("Publish failed" + ": " + String(err), "error");
    } finally {
      setPublishing(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Platforms</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            {PLATFORMS.map((p) => {
              const active = selected.has(p.id);
              return (
                <button
                  key={p.id}
                  onClick={() => togglePlatform(p.id)}
                  className={cn(
                    "flex flex-col items-center justify-center gap-2 rounded border px-4 py-4 text-sm font-medium transition-colors",
                    active
                      ? "border-midground bg-midground/10 text-midground"
                      : "border-border bg-card text-muted-foreground hover:bg-secondary",
                  )}
                >
                  <p.Icon className={cn("h-5 w-5", active ? p.color : "opacity-60")} />
                  <span>{p.label}</span>
                  <span
                    className={cn(
                      "rounded px-1.5 py-0.5 text-[10px]",
                      active ? "bg-background text-midground" : "bg-secondary text-muted-foreground",
                    )}
                  >
                    {active ? "On" : "Off"}
                  </span>
                </button>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {Array.from(selected).map((id) => {
        const platform = PLATFORMS.find((p) => p.id === id)!;
        return (
          <Card key={id}>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <platform.Icon className={cn("h-4 w-4", platform.color)} />
                {platform.label} post
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
              <textarea
                value={captions[id] ?? ""}
                onChange={(e) => setCaptions((c) => ({ ...c, [id]: e.target.value }))}
                placeholder={`Caption for ${platform.label}…`}
                rows={4}
                className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
              />
              <input
                type="text"
                value={tags[id] ?? ""}
                onChange={(e) => setTags((t) => ({ ...t, [id]: e.target.value }))}
                placeholder={`Tags for ${platform.label} (comma separated)`}
                className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
              />
              {status[id] && (
                <p className="text-xs text-muted-foreground">Status: {status[id]}</p>
              )}
            </CardContent>
          </Card>
        );
      })}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Calendar className="h-4 w-4" /> Schedule
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <input
            type="datetime-local"
            value={scheduledAt}
            onChange={(e) => setScheduledAt(e.target.value)}
            className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
          />
          <div className="flex items-center gap-2">
            <Button
              type="button"
              onClick={() => publish(false)}
              disabled={publishing || selected.size === 0 || !scheduledAt}
            >
              {publishing ? <Spinner className="mr-2" /> : <Calendar className="mr-2 h-4 w-4" />}
              Schedule
            </Button>
            <Button
              type="button"
              outlined
              onClick={() => publish(true)}
              disabled={publishing || selected.size === 0}
            >
              <Send className="mr-2 h-4 w-4" />
              Publish now
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
