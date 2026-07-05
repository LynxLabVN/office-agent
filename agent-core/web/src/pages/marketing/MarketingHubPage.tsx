import { useEffect } from "react";
import { Link } from "react-router-dom";
import {
  BarChart3,
  ClipboardList,
  Edit3,
  FileText,
  MessageSquare,
  Play,
  Send,
  ShoppingBag,
  Video,
} from "lucide-react";
import { Card, CardContent } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";

const MARKETING_CARDS = [
  {
    to: "/marketing/pipeline",
    icon: ClipboardList,
    title: "Pipeline",
    description: "Kanban board of content pieces from idea to published.",
    hint: "7 states",
  },
  {
    to: "/marketing/script",
    icon: FileText,
    title: "Script",
    description: "Hook generator and 9-section script editor.",
    hint: "per piece",
  },
  {
    to: "/marketing/review",
    icon: Play,
    title: "Review Queue",
    description: "Approve or request revisions before publishing.",
    hint: "manager gate",
  },
  {
    to: "/marketing/edit",
    icon: Edit3,
    title: "Edit & Preview",
    description: "Upload footage, build shotlists, render previews.",
    hint: "video pipeline",
  },
  {
    to: "/marketing/publish",
    icon: Send,
    title: "Publish",
    description: "Schedule or publish now across YT, IG, FB, TikTok.",
    hint: "multi-platform",
  },
  {
    to: "/marketing/comments",
    icon: MessageSquare,
    title: "Comments",
    description: "Inbox of platform comments with AI reply suggestions.",
    hint: "engage",
  },
  {
    to: "/marketing/analytics",
    icon: BarChart3,
    title: "Analytics",
    description: "Views, likes, and hooks leaderboard over time.",
    hint: "reports",
  },
  {
    to: "/marketing/catalog",
    icon: ShoppingBag,
    title: "Catalog",
    description: "Products, SKUs, and campaign references.",
    hint: "CRUD",
  },
];

const RECENT_ACTIVITY = [
  {
    initials: "MK",
    color: "bg-emerald-500",
    line: (
      <>
        <span className="font-medium">Maya K.</span> moved{" "}
        <span className="font-medium">"Summer Glow Routine"</span> to{" "}
        <span className="inline-flex items-center rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-500">
          Editing
        </span>
      </>
    ),
    meta: "Marketing · 12 min ago",
  },
  {
    initials: "LP",
    color: "bg-blue-500",
    line: (
      <>
        <span className="font-medium">Linh P.</span> submitted{" "}
        <span className="font-medium">"SPF Myths Debunked"</span> for review
      </>
    ),
    meta: "Marketing · 38 min ago",
  },
  {
    initials: "⚙",
    color: "bg-amber-500",
    line: (
      <>
        Cron job <span className="font-medium">nightly-tiktok-reply</span> ran successfully
      </>
    ),
    meta: "Agent · 1 h ago",
  },
];

export default function MarketingHubPage() {
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Marketing");
    setSubTitle("Content pipeline, review, publishing, and analytics.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {MARKETING_CARDS.map((card) => (
          <Link
            key={card.to}
            to={card.to}
            className="group block rounded-[var(--theme-radius)] border border-border bg-card p-5 transition-all hover:-translate-y-0.5 hover:border-midground/40 hover:shadow-lg"
          >
            <div
              className="mb-4 flex h-10 w-10 items-center justify-center rounded-lg"
              style={{
                backgroundColor: "color-mix(in srgb, var(--midground-base) 12%, transparent)",
                color: "var(--midground)",
              }}
            >
              <card.icon className="h-5 w-5" />
            </div>
            <div className="font-semibold text-card-foreground">{card.title}</div>
            <p className="mt-1 text-sm text-muted-foreground">{card.description}</p>
            <div className="mt-4 text-xs text-muted-foreground">{card.hint} →</div>
          </Link>
        ))}
      </div>

      <div>
        <h2 className="mb-3 text-sm font-semibold text-muted-foreground">Recent activity</h2>
        <Card>
          {RECENT_ACTIVITY.map((item, idx) => (
            <div
              key={idx}
              className={`flex items-center gap-3 px-4 py-3 text-sm ${
                idx < RECENT_ACTIVITY.length - 1 ? "border-b border-border" : ""
              }`}
            >
              <div
                className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-white ${item.color}`}
              >
                {item.initials}
              </div>
              <div className="min-w-0 flex-1">
                <div className="truncate">{item.line}</div>
                <div className="text-xs text-muted-foreground">{item.meta}</div>
              </div>
            </div>
          ))}
        </Card>
      </div>

      <Card>
        <CardContent className="py-12 text-center">
          <Video className="mx-auto mb-3 h-8 w-8 opacity-40" />
          <p className="text-sm font-medium text-muted-foreground">Start from the Pipeline</p>
          <p className="mt-1 text-xs text-text-tertiary">
            Drag content pieces through each stage until they are ready to publish.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
