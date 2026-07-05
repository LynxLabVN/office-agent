import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus, RefreshCw } from "lucide-react";
import { api } from "@/lib/api";
import type { MarketingPiece, MarketingPipelineResponse } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { usePageHeader } from "@/contexts/usePageHeader";
import { cn } from "@/lib/utils";

const DEFAULT_STATES = [
  "Idea",
  "Scripting",
  "Review",
  "Editing",
  "Rendering",
  "Scheduled",
  "Published",
];

const FORMAT_BADGE: Record<string, string> = {
  YT: "bg-red-500/15 text-red-500 border-red-500/30",
  IG: "bg-pink-500/15 text-pink-500 border-pink-500/30",
  FB: "bg-blue-500/15 text-blue-500 border-blue-500/30",
  TikTok: "bg-cyan-500/15 text-cyan-500 border-cyan-500/30",
};

function formatBadgeClass(format: string): string {
  return FORMAT_BADGE[format] ?? "bg-midground/10 text-midground border-midground/30";
}

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

interface KanbanCardProps {
  piece: MarketingPiece;
  onDragStart: (id: string) => void;
  onMove: (id: string, state: string) => void;
}

function KanbanCard({ piece, onDragStart, onMove }: KanbanCardProps) {
  return (
    <div
      draggable
      onDragStart={() => onDragStart(piece.id)}
      className="group cursor-grab rounded-[calc(var(--theme-radius)-2px)] border border-border bg-card p-3 shadow-sm transition-shadow hover:shadow-md active:cursor-grabbing"
    >
      {piece.thumbnail ? (
        <div
          className="mb-3 aspect-video w-full rounded bg-cover bg-center"
          style={{ backgroundImage: `url(${piece.thumbnail})` }}
        />
      ) : (
        <div
          className="mb-3 aspect-video w-full rounded"
          style={{
            background: `linear-gradient(135deg, ${avatarColor(piece.title).replace("bg-", "")}40, transparent)`,
          }}
        />
      )}
      <div className="mb-2 text-sm font-medium text-card-foreground">{piece.title}</div>
      <div className="mb-3 flex items-center gap-2">
        <span
          className={cn(
            "inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide",
            formatBadgeClass(piece.format),
          )}
        >
          {piece.format}
        </span>
        <span className="text-xs text-muted-foreground">{piece.due ?? "No due date"}</span>
      </div>
      <div className="flex items-center justify-between">
        <div
          className={cn(
            "flex h-6 w-6 items-center justify-center rounded-full text-[10px] font-semibold text-white",
            avatarColor(piece.owner),
          )}
          title={piece.owner}
        >
          {initials(piece.owner)}
        </div>
        <div className="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
          {DEFAULT_STATES.map(
            (s) =>
              s !== piece.state && (
                <button
                  key={s}
                  onClick={() => onMove(piece.id, s)}
                  className="h-5 rounded bg-secondary px-1.5 text-[10px] text-secondary-foreground hover:bg-secondary/80"
                  title={`Move to ${s}`}
                >
                  {s.slice(0, 2)}
                </button>
              ),
          )}
        </div>
      </div>
    </div>
  );
}

export default function PipelinePage() {
  const [data, setData] = useState<MarketingPipelineResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const { showToast } = useToast();
  const { setTitle, setSubTitle, setEnd } = usePageHeader();

  const states = useMemo(() => data?.states ?? DEFAULT_STATES, [data]);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    api.marketing
      .getPipeline()
      .then((res) => setData(res))
      .catch((err) => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    setTitle("Pipeline");
    setSubTitle("Drag cards between columns to update state.");
    setEnd(
      <Button type="button" ghost size="icon" onClick={load} disabled={loading}>
        {loading ? <Spinner /> : <RefreshCw className="h-4 w-4" />}
      </Button>,
    );
    return () => {
      setTitle("");
      setSubTitle("");
      setEnd(null);
    };
  }, [setTitle, setSubTitle, setEnd, load, loading]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
  }, [load]);

  const handleMove = useCallback(
    async (pieceId: string, newState: string) => {
      const prev = data;
      if (!prev) return;
      setData({
        ...prev,
        items: prev.items.map((i) => (i.id === pieceId ? { ...i, state: newState } : i)),
      });
      try {
        await api.marketing.movePiece(pieceId, newState);
        showToast(`Moved to ${newState}`, "success");
      } catch (err) {
        setData(prev);
        showToast("Move failed" + ": " + String(err), "error");
      }
    },
    [data, showToast],
  );

  const columns = useMemo(() => {
    return states.map((state) => ({
      state,
      items: data?.items.filter((i) => i.state === state) ?? [],
    }));
  }, [states, data]);

  if (loading && !data) {
    return (
      <div className="flex items-center justify-center py-24">
        <Spinner className="text-2xl text-primary" />
      </div>
    );
  }

  if (error && !data) {
    return (
      <div className="rounded border border-destructive/30 bg-destructive/10 p-6 text-center text-sm text-destructive">
        {error}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 overflow-x-auto pb-2">
        {columns.map(({ state, items }) => (
          <div
            key={state}
            onDragOver={(e) => {
              e.preventDefault();
              setDropTarget(state);
            }}
            onDragLeave={() => setDropTarget((cur) => (cur === state ? null : cur))}
            onDrop={(e) => {
              e.preventDefault();
              if (draggingId && dropTarget) {
                void handleMove(draggingId, dropTarget);
              }
              setDropTarget(null);
              setDraggingId(null);
            }}
            className={cn(
              "flex h-full min-h-[28rem] w-64 shrink-0 flex-col gap-3 rounded-[var(--theme-radius)] p-3 transition-colors",
              dropTarget === state
                ? "bg-midground/10 ring-1 ring-midground/30"
                : "bg-secondary/30",
            )}
          >
            <div className="flex items-center justify-between px-1 pb-1 text-xs font-semibold text-muted-foreground">
              <span>{state}</span>
              <span className="rounded bg-card px-1.5 py-0.5 text-[10px]">{items.length}</span>
            </div>
            <div className="flex flex-col gap-3">
              {items.map((piece) => (
                <KanbanCard
                  key={piece.id}
                  piece={piece}
                  onDragStart={setDraggingId}
                  onMove={handleMove}
                />
              ))}
            </div>
            <button
              onClick={() =>
                showToast(
                  "Use the agent to create a new content piece.",
                  "success",
                )
              }
              className="mt-auto flex items-center justify-center gap-1 rounded border border-dashed border-border py-2 text-xs text-muted-foreground hover:bg-card"
            >
              <Plus className="h-3.5 w-3.5" /> New piece
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
