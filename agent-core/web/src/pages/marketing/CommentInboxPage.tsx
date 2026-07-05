import { useEffect, useState } from "react";
import { Check, Flag, MessageSquare, Pencil, Send, Youtube, Facebook, Instagram } from "lucide-react";
import { api } from "@/lib/api";
import type { MarketingComment } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { Badge } from "@nous-research/ui/ui/components/badge";
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

const PLATFORM_TABS = [
  { id: "All", label: "All" },
  { id: "YT", label: "YouTube" },
  { id: "IG", label: "Instagram" },
  { id: "FB", label: "Facebook" },
  { id: "TikTok", label: "TikTok" },
];

const PLATFORM_META: Record<string, { label: string; Icon: React.ComponentType<{ className?: string }>; color: string }> = {
  YT: { label: "YouTube", Icon: Youtube, color: "text-red-500" },
  IG: { label: "Instagram", Icon: Instagram, color: "text-pink-500" },
  FB: { label: "Facebook", Icon: Facebook, color: "text-blue-500" },
  TikTok: { label: "TikTok", Icon: TikTokIcon, color: "text-cyan-500" },
};

function initials(name: string): string {
  return name
    .split(" ")
    .map((p) => p[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

function avatarColor(name: string): string {
  const colors = ["bg-emerald-500", "bg-blue-500", "bg-violet-500", "bg-amber-500", "bg-rose-500"];
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return colors[Math.abs(hash) % colors.length];
}

function timeAgo(value: string): string {
  const seconds = Math.floor((Date.now() - new Date(value).getTime()) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export default function CommentInboxPage() {
  const [platform, setPlatform] = useState("All");
  const [comments, setComments] = useState<MarketingComment[]>([]);
  const [loading, setLoading] = useState(true);
  const [replies, setReplies] = useState<Record<string, string>>({});
  const [sending, setSending] = useState<Record<string, boolean>>({});
  const [suggestions, setSuggestions] = useState<Record<string, string>>({});
  const [editing, setEditing] = useState<Record<string, boolean>>({});
  const [policies, setPolicies] = useState<Record<string, string>>({});
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Comments");
    setSubTitle("Platform inbox with AI-suggested replies.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const load = () => {
    setLoading(true);
    api.marketing
      .getComments(platform === "All" ? undefined : platform)
      .then((res) => {
        const list = res.comments ?? [];
        setComments(list);
        const pols: Record<string, string> = {};
        list.forEach((c) => {
          pols[c.id] = ["YT", "IG", "FB", "TikTok"].includes(c.platform) ? "auto-reply enabled" : "manual review";
        });
        setPolicies(pols);
      })
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [platform]);

  const suggestReply = (comment: MarketingComment) => {
    const suggestion = `Thanks for watching, ${comment.author}! Glad you enjoyed it.`;
    setSuggestions((s) => ({ ...s, [comment.id]: suggestion }));
    setReplies((r) => ({ ...r, [comment.id]: suggestion }));
    setEditing((e) => ({ ...e, [comment.id]: false }));
  };

  const applySuggestion = (comment: MarketingComment) => {
    const suggestion = suggestions[comment.id];
    if (suggestion) {
      setReplies((r) => ({ ...r, [comment.id]: suggestion }));
      setEditing((e) => ({ ...e, [comment.id]: false }));
    }
  };

  const sendReply = async (comment: MarketingComment) => {
    const text = replies[comment.id];
    if (!text) return;
    setSending((s) => ({ ...s, [comment.id]: true }));
    try {
      await api.marketing.replyToComment(comment.id, comment.platform, text);
      showToast("Reply sent", "success");
      setComments((prev) => prev.filter((c) => c.id !== comment.id));
    } catch (err) {
      showToast("Reply failed" + ": " + String(err), "error");
    } finally {
      setSending((s) => ({ ...s, [comment.id]: false }));
    }
  };

  const flagComment = (comment: MarketingComment) => {
    showToast(`Flagged comment from ${comment.author}`, "success");
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-2">
        {PLATFORM_TABS.map((p) => (
          <button
            key={p.id}
            onClick={() => setPlatform(p.id)}
            className={cn(
              "rounded px-3 py-1.5 text-xs font-medium transition-colors",
              platform === p.id
                ? "bg-midground text-background-base"
                : "border border-border bg-card text-muted-foreground hover:bg-secondary",
            )}
          >
            {p.label}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-24">
          <Spinner className="text-2xl text-primary" />
        </div>
      ) : comments.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-muted-foreground">
            <MessageSquare className="mx-auto mb-3 h-8 w-8 opacity-40" />
            <p className="text-sm font-medium">No comments</p>
            <p className="mt-1 text-xs text-text-tertiary">Check back later.</p>
          </CardContent>
        </Card>
      ) : (
        comments.map((comment) => {
          const meta = PLATFORM_META[comment.platform] ?? { label: comment.platform, Icon: MessageSquare, color: "text-midground" };
          const isEditing = editing[comment.id];
          return (
            <Card key={comment.id}>
              <CardHeader>
                <CardTitle className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-2">
                    <div
                      className={cn(
                        "flex h-8 w-8 items-center justify-center rounded-full text-xs font-semibold text-white",
                        avatarColor(comment.author),
                      )}
                    >
                      {initials(comment.author)}
                    </div>
                    <div className="flex flex-col">
                      <span className="font-medium">{comment.author}</span>
                      <span className="text-xs text-muted-foreground">{timeAgo(comment.posted_at)}</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge tone="outline" className="text-[10px]">
                      {policies[comment.id] ?? "manual review"}
                    </Badge>
                    <span className="flex items-center gap-1 text-xs text-muted-foreground">
                      <meta.Icon className={cn("h-3.5 w-3.5", meta.color)} />
                      {meta.label}
                    </span>
                  </div>
                </CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <p className="text-sm">{comment.text}</p>

                {suggestions[comment.id] && !isEditing && (
                  <div className="rounded bg-secondary/50 p-3 text-sm">
                    <p className="text-xs font-medium text-muted-foreground">AI-suggested reply</p>
                    <p className="mt-1">{suggestions[comment.id]}</p>
                    <div className="mt-2 flex items-center gap-2">
                      <Button type="button" size="sm" onClick={() => applySuggestion(comment)}>
                        <Check className="mr-1.5 h-3.5 w-3.5" /> Use
                      </Button>
                      <Button type="button" size="sm" outlined onClick={() => setEditing((e) => ({ ...e, [comment.id]: true }))}>
                        <Pencil className="mr-1.5 h-3.5 w-3.5" /> Edit
                      </Button>
                    </div>
                  </div>
                )}

                {(isEditing || !suggestions[comment.id]) && (
                  <textarea
                    value={replies[comment.id] ?? ""}
                    onChange={(e) => setReplies((r) => ({ ...r, [comment.id]: e.target.value }))}
                    placeholder="Write a reply…"
                    rows={3}
                    className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
                  />
                )}

                <div className="flex items-center gap-2">
                  <Button
                    type="button"
                    onClick={() => sendReply(comment)}
                    disabled={sending[comment.id] || !replies[comment.id]}
                  >
                    {sending[comment.id] ? <Spinner className="mr-2" /> : <Send className="mr-2 h-4 w-4" />}
                    Approve & Send
                  </Button>
                  <Button type="button" outlined onClick={() => suggestReply(comment)}>
                    Suggest reply
                  </Button>
                  <Button type="button" ghost onClick={() => flagComment(comment)}>
                    <Flag className="mr-2 h-4 w-4 text-muted-foreground" />
                    Flag
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })
      )}
    </div>
  );
}
