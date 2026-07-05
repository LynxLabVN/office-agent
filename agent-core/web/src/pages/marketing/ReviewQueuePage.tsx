import { useEffect, useMemo, useState } from "react";
import { Check, MessageSquare, Send } from "lucide-react";
import { api } from "@/lib/api";
import type { MarketingReviewItem } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";

export default function ReviewQueuePage() {
  const [items, setItems] = useState<MarketingReviewItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [feedback, setFeedback] = useState<Record<string, string>>({});
  const [acting, setActing] = useState<Record<string, boolean>>({});
  const [expanding, setExpanding] = useState<Record<string, boolean>>({});
  const [submitterFilter, setSubmitterFilter] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Review Queue");
    setSubTitle("Pending manager approvals and revision requests.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const load = () => {
    setLoading(true);
    api.marketing
      .getPendingReviews()
      .then((res) => setItems(res.items ?? []))
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filteredItems = useMemo(() => {
    return items.filter((item) => {
      if (submitterFilter && !item.submitted_by.toLowerCase().includes(submitterFilter.toLowerCase())) return false;
      if (dateFrom && new Date(item.submitted_at) < new Date(dateFrom)) return false;
      if (dateTo) {
        const to = new Date(dateTo);
        to.setHours(23, 59, 59, 999);
        if (new Date(item.submitted_at) > to) return false;
      }
      return true;
    });
  }, [items, submitterFilter, dateFrom, dateTo]);

  const approve = async (item: MarketingReviewItem) => {
    setActing((a) => ({ ...a, [item.id]: true }));
    try {
      await api.marketing.decideReview(item.piece_id, "approve");
      showToast("Approved", "success");
      setItems((prev) => prev.filter((i) => i.id !== item.id));
    } catch (err) {
      showToast("Action failed" + ": " + String(err), "error");
    } finally {
      setActing((a) => ({ ...a, [item.id]: false }));
    }
  };

  const requestRevision = async (item: MarketingReviewItem) => {
    setActing((a) => ({ ...a, [item.id]: true }));
    try {
      await api.marketing.decideReview(item.piece_id, "revise", feedback[item.id]);
      showToast("Revision requested", "success");
      setItems((prev) => prev.filter((i) => i.id !== item.id));
      setExpanding((e) => ({ ...e, [item.id]: false }));
    } catch (err) {
      showToast("Action failed" + ": " + String(err), "error");
    } finally {
      setActing((a) => ({ ...a, [item.id]: false }));
    }
  };

  const routeToTelegram = (item: MarketingReviewItem) => {
    const url = `https://t.me/share/url?url=${encodeURIComponent(window.location.origin + "/marketing/review")}&text=${encodeURIComponent(item.title)}`;
    window.open(url, "_blank", "noopener,noreferrer");
    showToast("Routed to Telegram", "success");
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <Spinner className="text-2xl text-primary" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardContent className="flex flex-col gap-3 py-4 sm:flex-row sm:items-end">
          <div className="flex-1">
            <Label className="text-xs">Submitter</Label>
            <Input
              value={submitterFilter}
              onChange={(e) => setSubmitterFilter(e.target.value)}
              placeholder="Filter by submitter"
            />
          </div>
          <div className="flex flex-1 gap-3">
            <div className="flex-1">
              <Label className="text-xs">From</Label>
              <input
                type="date"
                value={dateFrom}
                onChange={(e) => setDateFrom(e.target.value)}
                className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
              />
            </div>
            <div className="flex-1">
              <Label className="text-xs">To</Label>
              <input
                type="date"
                value={dateTo}
                onChange={(e) => setDateTo(e.target.value)}
                className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {filteredItems.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-muted-foreground">
            <Check className="mx-auto mb-3 h-8 w-8 opacity-40" />
            <p className="text-sm font-medium">No pending reviews</p>
            <p className="mt-1 text-xs text-text-tertiary">All caught up.</p>
          </CardContent>
        </Card>
      ) : (
        filteredItems.map((item) => {
          const expanded = expanding[item.id];
          return (
            <Card key={item.id}>
              <CardHeader>
                <CardTitle className="text-base">{item.title}</CardTitle>
                <p className="text-xs text-muted-foreground">
                  Submitted by {item.submitted_by} · {new Date(item.submitted_at).toLocaleString()}
                </p>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                {expanded && (
                  <>
                    <textarea
                      value={feedback[item.id] ?? ""}
                      onChange={(e) => setFeedback((f) => ({ ...f, [item.id]: e.target.value }))}
                      placeholder="Feedback for revision…"
                      rows={3}
                      className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
                    />
                    <div className="flex items-center gap-2">
                      <Button
                        type="button"
                        outlined
                        onClick={() => requestRevision(item)}
                        disabled={acting[item.id]}
                      >
                        {acting[item.id] ? <Spinner className="mr-2" /> : <Send className="mr-2 h-4 w-4" />}
                        Send back
                      </Button>
                      <Button
                        type="button"
                        ghost
                        onClick={() => setExpanding((e) => ({ ...e, [item.id]: false }))}
                      >
                        Cancel
                      </Button>
                    </div>
                  </>
                )}

                <div className="flex items-center gap-2">
                  <Button
                    type="button"
                    onClick={() => approve(item)}
                    disabled={acting[item.id]}
                  >
                    {acting[item.id] ? <Spinner className="mr-2" /> : <Check className="mr-2 h-4 w-4" />}
                    Approve
                  </Button>
                  {!expanded && (
                    <Button
                      type="button"
                      outlined
                      onClick={() => setExpanding((e) => ({ ...e, [item.id]: true }))}
                      disabled={acting[item.id]}
                    >
                      <MessageSquare className="mr-2 h-4 w-4" />
                      Revise
                    </Button>
                  )}
                  <Button
                    type="button"
                    ghost
                    onClick={() => routeToTelegram(item)}
                  >
                    <Send className="mr-2 h-4 w-4 text-muted-foreground" />
                    Route to Telegram
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
