import { useEffect, useState } from "react";
import { TrendingUp, Users } from "lucide-react";
import { api } from "@/lib/api";
import type { HRAnalyticsOverview } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";

const CHART_HEIGHT = 160;

export default function HRAnalyticsPage() {
  const [data, setData] = useState<HRAnalyticsOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [days, setDays] = useState(30);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Analytics");
    setSubTitle("Time-to-hire, funnel, and source effectiveness.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const load = () => {
    setLoading(true);
    const dateTo = new Date().toISOString().split("T")[0];
    const dateFrom = new Date(Date.now() - days * 86400000).toISOString().split("T")[0];
    api.hr
      .getAnalyticsOverview(dateFrom, dateTo)
      .then(setData)
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [days]);

  const funnelEntries = data ? Object.entries(data.funnel) : [];
  const sourceEntries = data ? Object.entries(data.source_effectiveness) : [];
  const maxFunnel = Math.max(...funnelEntries.map(([, v]) => v), 1);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center gap-2">
        {[7, 30, 90].map((d) => (
          <Button
            key={d}
            type="button"
            size="sm"
            outlined={days !== d}
            onClick={() => setDays(d)}
          >
            {d}d
          </Button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-24">
          <Spinner className="text-2xl text-primary" />
        </div>
      ) : !data ? null : (
        <>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">Time to hire</div>
                <div className="mt-1 text-2xl font-semibold text-card-foreground">
                  {data.time_to_hire_days.toFixed(1)}d
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">Total applied</div>
                <div className="mt-1 text-2xl font-semibold text-card-foreground">
                  {data.funnel.Applied ?? 0}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">Hired</div>
                <div className="mt-1 text-2xl font-semibold text-card-foreground">
                  {data.funnel.Hired ?? 0}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">Top source</div>
                <div className="mt-1 text-2xl font-semibold text-card-foreground">
                  {sourceEntries[0]?.[0] ?? "—"}
                </div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Users className="h-5 w-5 text-muted-foreground" />
                <CardTitle className="text-base">Hiring funnel</CardTitle>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-end gap-2" style={{ height: CHART_HEIGHT }}>
                {funnelEntries.map(([stage, count]) => {
                  const h = Math.round((count / maxFunnel) * CHART_HEIGHT);
                  return (
                    <div key={stage} className="flex flex-1 flex-col items-center gap-2">
                      <div
                        className="w-full rounded-t bg-midground/60"
                        style={{ height: Math.max(h, 4) }}
                      />
                      <span className="text-center text-[10px] text-muted-foreground">{stage}</span>
                      <span className="text-xs font-medium">{count}</span>
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <TrendingUp className="h-5 w-5 text-muted-foreground" />
                <CardTitle className="text-base">Source effectiveness</CardTitle>
              </div>
            </CardHeader>
            <CardContent>
              {sourceEntries.length === 0 ? (
                <p className="text-sm text-muted-foreground">No source data.</p>
              ) : (
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border text-xs text-muted-foreground">
                      <th className="py-2 text-left font-medium">Source</th>
                      <th className="py-2 text-right font-medium">Hires</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sourceEntries.map(([source, count]) => (
                      <tr key={source} className="border-b border-border/50">
                        <td className="py-2 pr-4">{source}</td>
                        <td className="py-2 text-right">{count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
