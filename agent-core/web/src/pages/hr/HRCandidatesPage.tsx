import { useEffect, useMemo, useState } from "react";
import { FileText, GitCompare, Search, User, X } from "lucide-react";
import { api } from "@/lib/api";
import type { HRCandidate } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Input } from "@nous-research/ui/ui/components/input";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

interface SubScore {
  label: string;
  score: number; // 0–1
}

const MOCK_SUB_SCORES: SubScore[] = [
  { label: "JD match", score: 0.82 },
  { label: "Experience", score: 0.75 },
  { label: "Skills", score: 0.9 },
  { label: "Communication", score: 0.7 },
];

export default function HRCandidatesPage() {
  const [candidates, setCandidates] = useState<HRCandidate[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [jobId, setJobId] = useState("");
  const [query, setQuery] = useState("");
  const [viewingCv, setViewingCv] = useState<HRCandidate | null>(null);
  const [cvText, setCvText] = useState<string | null>(null);
  const [cvLoading, setCvLoading] = useState(false);
  const [compareOpen, setCompareOpen] = useState(false);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Candidates");
    setSubTitle("CV viewer, scores, and comparison.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const load = () => {
    setLoading(true);
    api.hr
      .getCandidates()
      .then((res) => setCandidates(res.candidates ?? []))
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggle = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const compare = async () => {
    if (selectedIds.size < 2 || !jobId) return;
    try {
      await api.hr.compareCandidates(Array.from(selectedIds), jobId);
      setCompareOpen(true);
      showToast("Comparison generated", "success");
    } catch (err) {
      showToast("Compare failed" + ": " + String(err), "error");
    }
  };

  const screen = async () => {
    if (!jobId) return;
    try {
      await api.hr.screenCandidates(jobId);
      showToast("Screening started", "success");
      load();
    } catch (err) {
      showToast("Screen failed" + ": " + String(err), "error");
    }
  };

  const viewCv = async (candidate: HRCandidate) => {
    setViewingCv(candidate);
    setCvLoading(true);
    setCvText(null);
    try {
      const res = await api.hr.getCandidateCv(candidate.id);
      const text = await res.text();
      setCvText(text);
    } catch (err) {
      showToast("CV load failed" + ": " + String(err), "error");
      setCvText("Unable to load CV.");
    } finally {
      setCvLoading(false);
    }
  };

  const filtered = candidates.filter(
    (c) =>
      c.name.toLowerCase().includes(query.toLowerCase()) ||
      c.email.toLowerCase().includes(query.toLowerCase()),
  );

  const selectedCandidates = useMemo(
    () => candidates.filter((c) => selectedIds.has(c.id)),
    [candidates, selectedIds],
  );

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardContent className="flex flex-wrap items-center gap-2 py-4">
          <div className="relative min-w-[200px] flex-1">
            <Search className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search candidates…"
              className="pl-9"
            />
          </div>
          <Input
            value={jobId}
            onChange={(e) => setJobId(e.target.value)}
            placeholder="Job ID"
            className="w-40"
          />
          <Button type="button" outlined onClick={screen} disabled={!jobId}>
            Screen all
          </Button>
          <Button type="button" outlined onClick={compare} disabled={selectedIds.size < 2 || !jobId}>
            <GitCompare className="mr-2 h-4 w-4" />
            Compare
          </Button>
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
        <div>
          {loading ? (
            <div className="flex items-center justify-center py-24">
              <Spinner className="text-2xl text-primary" />
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2">
              {filtered.map((c) => (
                <Card
                  key={c.id}
                  className={cn(
                    "cursor-pointer transition-colors",
                    selectedIds.has(c.id) ? "border-midground ring-1 ring-midground" : "",
                  )}
                  onClick={() => toggle(c.id)}
                >
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-secondary">
                        <User className="h-4 w-4" />
                      </div>
                      {c.name}
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="text-sm">
                    <p className="text-muted-foreground">{c.email}</p>
                    <p className="text-xs text-text-tertiary">{c.source}</p>
                    {c.score !== null && c.score !== undefined && (
                      <div className="mt-3">
                        <div className="mb-1 flex items-center justify-between text-xs text-muted-foreground">
                          <span>Match score</span>
                          <span>{(c.score * 100).toFixed(0)}%</span>
                        </div>
                        <div className="h-2 w-full rounded bg-secondary">
                          <div
                            className="h-2 rounded bg-midground"
                            style={{ width: `${Math.min(c.score * 100, 100)}%` }}
                          />
                        </div>
                      </div>
                    )}
                    <div className="mt-4 space-y-2">
                      {MOCK_SUB_SCORES.map((sub) => (
                        <div key={sub.label}>
                          <div className="mb-1 flex items-center justify-between text-[10px] text-muted-foreground">
                            <span>{sub.label}</span>
                            <span>{(sub.score * 100).toFixed(0)}%</span>
                          </div>
                          <div className="h-1.5 w-full rounded bg-secondary">
                            <div
                              className="h-1.5 rounded bg-midground/70"
                              style={{ width: `${Math.min(sub.score * 100, 100)}%` }}
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                    <Button
                      type="button"
                      ghost
                      size="sm"
                      className="mt-4"
                      onClick={(e) => {
                        e.stopPropagation();
                        void viewCv(c);
                      }}
                    >
                      <FileText className="mr-2 h-4 w-4" />
                      View CV
                    </Button>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">CV viewer</CardTitle>
            </CardHeader>
            <CardContent>
              {!viewingCv ? (
                <p className="text-sm text-muted-foreground">Select a candidate to view their CV.</p>
              ) : cvLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Spinner className="text-primary" />
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{viewingCv.name}</span>
                    <Button type="button" ghost size="icon" onClick={() => setViewingCv(null)}>
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                  <pre className="max-h-[60vh] overflow-auto rounded border border-border bg-secondary/30 p-3 text-xs whitespace-pre-wrap">
                    {cvText}
                  </pre>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>

      {compareOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="max-h-[80vh] w-full max-w-4xl overflow-auto">
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm">Compare candidates</CardTitle>
                <Button type="button" ghost size="icon" onClick={() => setCompareOpen(false)}>
                  <X className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border text-xs text-muted-foreground">
                    <th className="py-2 text-left font-medium">Metric</th>
                    {selectedCandidates.map((c) => (
                      <th key={c.id} className="py-2 text-left font-medium">{c.name}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b border-border/50">
                    <td className="py-2 text-muted-foreground">Score</td>
                    {selectedCandidates.map((c) => (
                      <td key={c.id} className="py-2">{c.score != null ? (c.score * 100).toFixed(0) + "%" : "—"}</td>
                    ))}
                  </tr>
                  {MOCK_SUB_SCORES.map((sub) => (
                    <tr key={sub.label} className="border-b border-border/50">
                      <td className="py-2 text-muted-foreground">{sub.label}</td>
                      {selectedCandidates.map((c) => (
                        <td key={c.id} className="py-2">
                          <div className="flex items-center gap-2">
                            <div className="h-1.5 w-16 rounded bg-secondary">
                              <div
                                className="h-1.5 rounded bg-midground"
                                style={{ width: `${Math.min(sub.score * 100, 100)}%` }}
                              />
                            </div>
                            <span className="text-xs">{(sub.score * 100).toFixed(0)}%</span>
                          </div>
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
