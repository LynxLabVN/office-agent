import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { Users } from "lucide-react";
import { api } from "@/lib/api";
import type { HRApplication } from "@/lib/api";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

const STAGES = ["Applied", "Screened", "Interview", "Offer", "Hired", "Rejected"];

export default function HRPipelinePage() {
  const { jobId } = useParams<{ jobId: string }>();
  const [applications, setApplications] = useState<HRApplication[]>([]);
  const [stages, setStages] = useState<string[]>(STAGES);
  const [loading, setLoading] = useState(true);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle(`Pipeline · ${jobId ?? "all jobs"}`);
    setSubTitle("Move candidates through the hiring stages.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle, jobId]);

  const load = () => {
    setLoading(true);
    api.hr
      .getPipeline(jobId)
      .then((res) => {
        setApplications(res.applications ?? []);
        if (res.stages?.length) setStages(res.stages);
      })
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [jobId]);

  const move = async (applicationId: string, newStage: string) => {
    const prev = applications;
    setApplications((apps) =>
      apps.map((a) => (a.id === applicationId ? { ...a, stage: newStage } : a)),
    );
    try {
      await api.hr.moveApplication(applicationId, newStage);
      showToast(`Moved to ${newStage}`, "success");
    } catch (err) {
      setApplications(prev);
      showToast("Move failed" + ": " + String(err), "error");
    }
  };

  const columns = useMemo(() => {
    return stages.map((stage) => ({
      stage,
      items: applications.filter((a) => a.stage === stage),
    }));
  }, [stages, applications]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <Spinner className="text-2xl text-primary" />
      </div>
    );
  }

  return (
    <div className="flex items-start gap-2 overflow-x-auto pb-2">
      {columns.map(({ stage, items }) => (
        <div
          key={stage}
          onDragOver={(e) => {
            e.preventDefault();
            setDropTarget(stage);
          }}
          onDragLeave={() => setDropTarget((cur) => (cur === stage ? null : cur))}
          onDrop={(e) => {
            e.preventDefault();
            if (draggingId && dropTarget) void move(draggingId, dropTarget);
            setDropTarget(null);
            setDraggingId(null);
          }}
          className={cn(
            "flex h-full min-h-[28rem] w-64 shrink-0 flex-col gap-3 rounded-[var(--theme-radius)] p-3 transition-colors",
            dropTarget === stage ? "bg-midground/10 ring-1 ring-midground/30" : "bg-secondary/30",
          )}
        >
          <div className="flex items-center justify-between px-1 pb-1 text-xs font-semibold text-muted-foreground">
            <span>{stage}</span>
            <span className="rounded bg-card px-1.5 py-0.5 text-[10px]">{items.length}</span>
          </div>
          <div className="flex flex-col gap-3">
            {items.map((app) => (
              <div
                key={app.id}
                draggable
                onDragStart={() => setDraggingId(app.id)}
                className="cursor-grab rounded border border-border bg-card p-3 shadow-sm transition-shadow hover:shadow-md active:cursor-grabbing"
              >
                <div className="mb-2 flex h-8 w-8 items-center justify-center rounded-full bg-secondary text-xs font-semibold">
                  <Users className="h-4 w-4" />
                </div>
                <div className="text-sm font-medium text-card-foreground">{app.candidate_id}</div>
                <div className="text-xs text-muted-foreground">
                  Applied {new Date(app.applied_at).toLocaleDateString()}
                </div>
                <div className="mt-2 flex flex-wrap gap-1">
                  {stages.map(
                    (s) =>
                      s !== app.stage && (
                        <button
                          key={s}
                          onClick={() => move(app.id, s)}
                          className="rounded bg-secondary px-1.5 py-0.5 text-[10px] text-secondary-foreground hover:bg-secondary/80"
                        >
                          {s.slice(0, 2)}
                        </button>
                      ),
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
