import { useEffect, useState } from "react";
import { BarChart3, TrendingUp } from "lucide-react";
import { api } from "@/lib/api";
import type {
  MarketingAnalyticsOverview,
  MarketingHookLeaderboardEntry,
} from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";

const CHART_HEIGHT = 160;

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export default function MAnalyticsPage() {
  const [overview, setOverview] = useState<MarketingAnalyticsOverview | null>(null);
  const [leaderboard, setLeaderboard] = useState<MarketingHookLeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [days, setDays] = useState(30);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Analytics");
    setSubTitle("Views, likes, and hooks that worked.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const load = () => {
    setLoading(true);
    const dateTo = new Date().toISOString().split("T")[0];
    const dateFrom = new Date(Date.now() - days * 86400000).toISOString().split("T")[0];
    Promise.all([
      api.marketing.getAnalyticsOverview(dateFrom, dateTo).catch((err) => {
        showToast("Overview failed" + ": " + String(err), "error");
        return null;
      }),
      api.marketing.getHooksLeaderboard().catch((err) => {
        showToast("Leaderboard failed" + ": " + String(err), "error");
        return { leaderboard: [] };
      }),
    ]).then(([ov, lb]) => {
      if (ov) setOverview(ov);
      setLeaderboard(lb.leaderboard ?? []);
      setLoading(false);
    });
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [days]);

  const stats = overview
    ? [
        { label: "Views", value: overview.views },
        { label: "Likes", value: overview.likes },
        { label: "Comments", value: overview.comments },
        { label: "Shares", value: overview.shares },
      ]
    : [];

  const maxValue = Math.max(...stats.map((s) => s.value), 1);

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
      ) : (
        <>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {stats.map((s) => (
              <Card key={s.label}>
                <CardContent className="py-5">
                  <div className="text-xs text-muted-foreground">{s.label}</div>
                  <div className="mt-1 text-2xl font-semibold text-card-foreground">
                    {formatNumber(s.value)}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>

          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <BarChart3 className="h-5 w-5 text-muted-foreground" />
                <CardTitle className="text-base">Engagement overview</CardTitle>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-end gap-1" style={{ height: CHART_HEIGHT }}>
                {stats.map((s) => {
                  const h = Math.round((s.value / maxValue) * CHART_HEIGHT);
                  return (
                    <div key={s.label} className="flex flex-1 flex-col items-center gap-2">
                      <div
                        className="w-full rounded-t bg-midground/60"
                        style={{ height: Math.max(h, 4) }}
                      />
                      <span className="text-xs text-muted-foreground">{s.label}</span>
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
                <CardTitle className="text-base">Hooks leaderboard</CardTitle>
              </div>
            </CardHeader>
            <CardContent>
              {leaderboard.length === 0 ? (
                <p className="text-sm text-muted-foreground">No hook data yet.</p>
              ) : (
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border text-xs text-muted-foreground">
                      <th className="py-2 text-left font-medium">Hook</th>
                      <th className="py-2 text-right font-medium">Uses</th>
                      <th className="py-2 text-right font-medium">Avg views</th>
                    </tr>
                  </thead>
                  <tbody>
                    {leaderboard.map((row, idx) => (
                      <tr key={idx} className="border-b border-border/50">
                        <td className="py-2 pr-4">{row.hook}</td>
                        <td className="py-2 pr-4 text-right">{row.uses}</td>
                        <td className="py-2 text-right">{formatNumber(row.avg_views)}</td>
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
