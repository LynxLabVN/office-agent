import { useEffect, useMemo, useState } from "react";
import { Briefcase, Pencil, Plus, Send, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import type { HRJob } from "@/lib/api";
import { Markdown } from "@/components/Markdown";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

const BOARDS = [
  { id: "linkedin", label: "LinkedIn", limit: 3000 },
  { id: "topdev", label: "TopDev", limit: 2000 },
  { id: "vietnamworks", label: "VietnamWorks", limit: 2500 },
  { id: "itviec", label: "ITviec", limit: 2000 },
  { id: "careers", label: "Own careers page", limit: 5000 },
];

function emptyJob(): HRJob {
  return { id: "", title: "", department: "", location: "", status: "open", description: "" };
}

export default function HRJobsPage() {
  const [jobs, setJobs] = useState<HRJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<HRJob | null>(null);
  const [previewTab, setPreviewTab] = useState<"edit" | "preview">("edit");
  const [selectedBoards, setSelectedBoards] = useState<Set<string>>(new Set());
  const [boardText, setBoardText] = useState("");
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [posting, setPosting] = useState(false);
  const { showToast } = useToast();
  const { setTitle, setSubTitle, setEnd } = usePageHeader();

  useEffect(() => {
    setTitle("Jobs");
    setSubTitle("Job descriptions and posting console.");
    setEnd(
      <Button type="button" size="sm" onClick={() => { setEditing(emptyJob()); setPreviewTab("edit"); }}>
        <Plus className="mr-2 h-4 w-4" />
        New job
      </Button>,
    );
    return () => {
      setTitle("");
      setSubTitle("");
      setEnd(null);
    };
  }, [setTitle, setSubTitle, setEnd]);

  const load = () => {
    setLoading(true);
    api.hr
      .getJobs()
      .then((res) => setJobs(res.jobs ?? []))
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const activeJob = useMemo(
    () => jobs.find((j) => j.id === activeJobId) || null,
    [jobs, activeJobId],
  );

  const generatedPreview = useMemo(() => {
    if (!activeJob) return "";
    const base = `${activeJob.title}\n${activeJob.department} · ${activeJob.location}\n\n${activeJob.description ?? ""}`.trim();
    return base;
  }, [activeJob]);

  const remainingChars = useMemo(() => {
    const board = BOARDS.find((b) => b.id === Array.from(selectedBoards)[0]);
    if (!board) return null;
    return board.limit - (boardText || generatedPreview).length;
  }, [selectedBoards, boardText, generatedPreview]);

  const save = async () => {
    if (!editing) return;
    setSaving(true);
    try {
      if (editing.id) {
        await api.hr.updateJob(editing.id, editing);
        showToast("Job updated", "success");
      } else {
        await api.hr.createJob(editing);
        showToast("Job created", "success");
      }
      setEditing(null);
      load();
    } catch (err) {
      showToast("Save failed" + ": " + String(err), "error");
    } finally {
      setSaving(false);
    }
  };

  const remove = async (id: string) => {
    try {
      await api.hr.deleteJob(id);
      showToast("Job deleted", "success");
      load();
    } catch (err) {
      showToast("Delete failed" + ": " + String(err), "error");
    }
  };

  const post = async () => {
    if (!activeJobId || selectedBoards.size === 0) return;
    setPosting(true);
    try {
      await api.hr.postJob(activeJobId, Array.from(selectedBoards));
      showToast("Job posted to " + Array.from(selectedBoards).join(", "), "success");
    } catch (err) {
      showToast("Post failed" + ": " + String(err), "error");
    } finally {
      setPosting(false);
    }
  };

  const toggleBoard = (id: string) => {
    setSelectedBoards((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="flex flex-col gap-4">
      {editing && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">{editing.id ? "Edit job" : "New job"}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 sm:grid-cols-2">
            <div>
              <Label className="text-xs">Title</Label>
              <Input
                value={editing.title}
                onChange={(e) => setEditing((j) => ({ ...j!, title: e.target.value }))}
                placeholder="Job title"
              />
            </div>
            <div>
              <Label className="text-xs">Department</Label>
              <Input
                value={editing.department}
                onChange={(e) => setEditing((j) => ({ ...j!, department: e.target.value }))}
                placeholder="Department"
              />
            </div>
            <div>
              <Label className="text-xs">Location</Label>
              <Input
                value={editing.location}
                onChange={(e) => setEditing((j) => ({ ...j!, location: e.target.value }))}
                placeholder="Location"
              />
            </div>
            <div>
              <Label className="text-xs">Status</Label>
              <select
                className="w-full rounded border border-border bg-background px-2 py-1.5 text-sm"
                value={editing.status}
                onChange={(e) => setEditing((j) => ({ ...j!, status: e.target.value as HRJob["status"] }))}
              >
                <option value="open">Open</option>
                <option value="paused">Paused</option>
                <option value="closed">Closed</option>
              </select>
            </div>
            <div className="sm:col-span-2">
              <div className="mb-2 flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setPreviewTab("edit")}
                  className={cn("text-xs font-medium", previewTab === "edit" ? "text-foreground" : "text-muted-foreground")}
                >
                  JD Markdown
                </button>
                <button
                  type="button"
                  onClick={() => setPreviewTab("preview")}
                  className={cn("text-xs font-medium", previewTab === "preview" ? "text-foreground" : "text-muted-foreground")}
                >
                  Preview
                </button>
              </div>
              {previewTab === "edit" ? (
                <textarea
                  value={editing.description ?? ""}
                  onChange={(e) => setEditing((j) => ({ ...j!, description: e.target.value }))}
                  placeholder="# Job description\n\n## Responsibilities\n…"
                  rows={10}
                />
              ) : (
                <div className="rounded border border-border bg-card p-4">
                  <Markdown content={editing.description || "*No description yet.*"} />
                </div>
              )}
            </div>
            <div className="flex gap-2 sm:col-span-2">
              <Button type="button" onClick={save} disabled={saving}>
                {saving ? <Spinner className="mr-2" /> : <Briefcase className="mr-2 h-4 w-4" />}
                Save
              </Button>
              <Button type="button" outlined onClick={() => setEditing(null)}>
                Cancel
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Post to boards</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-4">
          <div className="flex flex-wrap gap-2">
            {jobs.map((j) => (
              <button
                key={j.id}
                type="button"
                onClick={() => setActiveJobId(j.id)}
                className={cn(
                  "rounded border px-3 py-1 text-xs transition-colors",
                  activeJobId === j.id
                    ? "border-midground bg-midground/10 text-midground"
                    : "border-border bg-card hover:bg-secondary",
                )}
              >
                {j.title || j.id}
              </button>
            ))}
          </div>

          {activeJob && (
            <>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {BOARDS.map((b) => (
                  <label
                    key={b.id}
                    className={cn(
                      "flex cursor-pointer items-center gap-2 rounded border p-3 text-sm transition-colors",
                      selectedBoards.has(b.id)
                        ? "border-midground bg-midground/10"
                        : "border-border bg-card hover:bg-secondary",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={selectedBoards.has(b.id)}
                      onChange={() => toggleBoard(b.id)}
                      className="h-4 w-4"
                    />
                    <span className="flex-1">{b.label}</span>
                    <span className="text-[10px] text-muted-foreground">{b.limit}</span>
                  </label>
                ))}
              </div>

              <div>
                <Label className="text-xs">Board-text preview</Label>
                <textarea
                  value={boardText}
                  onChange={(e) => setBoardText(e.target.value)}
                  placeholder={generatedPreview}
                  rows={6}
                />
                <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
                  <span>Defaults to JD when empty.</span>
                  {remainingChars !== null && (
                    <span className={remainingChars < 0 ? "text-destructive" : ""}>
                      {remainingChars} chars remaining
                    </span>
                  )}
                </div>
              </div>

              <div className="flex gap-2">
                <Button type="button" onClick={post} disabled={posting || selectedBoards.size === 0}>
                  {posting ? <Spinner className="mr-2" /> : <Send className="mr-2 h-4 w-4" />}
                  Post to {selectedBoards.size} board{selectedBoards.size === 1 ? "" : "s"}
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <div className="flex items-center justify-center py-24">
              <Spinner className="text-2xl text-primary" />
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-xs text-muted-foreground">
                  <th className="py-3 pl-4 text-left font-medium">Title</th>
                  <th className="py-3 text-left font-medium">Department</th>
                  <th className="py-3 text-left font-medium">Location</th>
                  <th className="py-3 text-left font-medium">Status</th>
                  <th className="py-3 pr-4 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {jobs.map((j) => (
                  <tr key={j.id} className="border-b border-border/50 hover:bg-secondary/20">
                    <td className="py-3 pl-4 font-medium">{j.title}</td>
                    <td className="py-3">{j.department}</td>
                    <td className="py-3">{j.location}</td>
                    <td className="py-3">
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase ${
                          j.status === "open"
                            ? "bg-emerald-500/15 text-emerald-500"
                            : j.status === "paused"
                            ? "bg-amber-500/15 text-amber-500"
                            : "bg-muted text-muted-foreground"
                        }`}
                      >
                        {j.status}
                      </span>
                    </td>
                    <td className="py-3 pr-4 text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          type="button"
                          ghost
                          size="icon"
                          onClick={() => { setActiveJobId(j.id); window.scrollTo({ top: 0, behavior: "smooth" }); }}
                          title="Post to boards"
                        >
                          <Send className="h-4 w-4" />
                        </Button>
                        <Button type="button" ghost size="icon" onClick={() => { setEditing(j); setPreviewTab("edit"); }}>
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button type="button" ghost size="icon" onClick={() => remove(j.id)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
