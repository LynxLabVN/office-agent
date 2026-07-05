import { useEffect, useMemo, useState } from "react";
import { Mail, MessageSquare, Phone, Send, User } from "lucide-react";
import { api } from "@/lib/api";
import type { HRCandidate, HRCommsMessage } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

const CHANNELS = [
  { id: "all", label: "All", icon: MessageSquare },
  { id: "zalo", label: "Zalo", icon: Phone },
  { id: "telegram", label: "Telegram", icon: Send },
  { id: "email", label: "Email", icon: Mail },
];

const TEMPLATES = [
  { label: "Interview reminder", text: "Hi {name}, this is a friendly reminder about your interview. Please reply if you need to reschedule." },
  { label: "Offer letter", text: "Hi {name}, we're excited to offer you the position. Please review the attached details and let us know your decision." },
  { label: "Rejection", text: "Hi {name}, thank you for your interest. We've decided to move forward with other candidates at this time." },
  { label: "Screening invite", text: "Hi {name}, we'd like to invite you for an initial screening call. Please reply with your availability." },
];

export default function HRCommsPage() {
  const [channel, setChannel] = useState("all");
  const [candidates, setCandidates] = useState<HRCandidate[]>([]);
  const [selectedCandidateId, setSelectedCandidateId] = useState<string>("");
  const [candidateQuery, setCandidateQuery] = useState("");
  const [messages, setMessages] = useState<HRCommsMessage[]>([]);
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Comms");
    setSubTitle("Candidate threads by channel.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const loadCandidates = () => {
    api.hr
      .getCandidates()
      .then((res) => setCandidates(res.candidates ?? []))
      .catch((err) => showToast("Candidates load failed" + ": " + String(err), "error"));
  };

  useEffect(() => {
    loadCandidates();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadMessages = () => {
    if (!selectedCandidateId) {
      setMessages([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    api.hr
      .getCommsLog(selectedCandidateId, channel === "all" ? undefined : channel)
      .then((res) => setMessages(res.messages ?? []))
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadMessages();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channel, selectedCandidateId]);

  const send = async () => {
    if (!selectedCandidateId || !text || channel === "all") return;
    setSending(true);
    try {
      await api.hr.sendComms(selectedCandidateId, channel, text);
      showToast("Message sent", "success");
      setText("");
      loadMessages();
    } catch (err) {
      showToast("Send failed" + ": " + String(err), "error");
    } finally {
      setSending(false);
    }
  };

  const selectedCandidate = useMemo(
    () => candidates.find((c) => c.id === selectedCandidateId),
    [candidates, selectedCandidateId],
  );

  const applyTemplate = (templateText: string) => {
    const name = selectedCandidate?.name ?? "Candidate";
    setText(templateText.replace(/\{name\}/g, name));
  };

  const filteredCandidates = candidates.filter(
    (c) =>
      c.name.toLowerCase().includes(candidateQuery.toLowerCase()) ||
      c.email.toLowerCase().includes(candidateQuery.toLowerCase()),
  );

  return (
    <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
      <div className="flex flex-col gap-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Candidates</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-2">
            <Input
              value={candidateQuery}
              onChange={(e) => setCandidateQuery(e.target.value)}
              placeholder="Search candidates…"
            />
            <div className="flex flex-col gap-1">
              {filteredCandidates.map((c) => (
                <button
                  key={c.id}
                  onClick={() => setSelectedCandidateId(c.id)}
                  className={cn(
                    "flex items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors",
                    selectedCandidateId === c.id ? "bg-midground/10 text-midground" : "hover:bg-secondary",
                  )}
                >
                  <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-secondary">
                    <User className="h-3.5 w-3.5" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate">{c.name}</div>
                    <div className="truncate text-[10px] text-muted-foreground">{c.email}</div>
                  </div>
                </button>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Channel</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-1">
            {CHANNELS.map((c) => (
              <button
                key={c.id}
                onClick={() => setChannel(c.id)}
                className={cn(
                  "flex items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors",
                  channel === c.id ? "bg-midground/10 text-midground" : "hover:bg-secondary",
                )}
              >
                <c.icon className="h-4 w-4" />
                {c.label}
              </button>
            ))}
          </CardContent>
        </Card>
      </div>

      <div className="flex flex-col gap-4">
        <Card className="flex-1">
          <CardHeader>
            <CardTitle className="text-sm">
              {selectedCandidate ? `Thread · ${selectedCandidate.name}` : "Thread"}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex items-center justify-center py-24">
                <Spinner className="text-2xl text-primary" />
              </div>
            ) : !selectedCandidate ? (
              <p className="text-sm text-muted-foreground">Select a candidate to view their thread.</p>
            ) : messages.length === 0 ? (
              <p className="text-sm text-muted-foreground">No messages yet.</p>
            ) : (
              <div className="flex flex-col gap-3">
                {messages.map((m) => {
                  const isInbound = m.channel === "email";
                  return (
                    <div
                      key={m.id}
                      className={cn(
                        "flex max-w-[80%] flex-col gap-1 rounded-lg p-3 text-sm",
                        isInbound
                          ? "self-start bg-secondary text-secondary-foreground"
                          : "self-end bg-midground/10 text-midground",
                      )}
                    >
                      <p>{m.text}</p>
                      <span className="text-[10px] opacity-70">
                        {m.channel} · {new Date(m.sent_at).toLocaleString()}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Templates</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2">
            {TEMPLATES.map((t) => (
              <button
                key={t.label}
                onClick={() => applyTemplate(t.text)}
                className="rounded border border-border px-3 py-1.5 text-left text-xs hover:bg-secondary"
              >
                {t.label}
              </button>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardContent className="flex flex-col gap-2 py-4">
            <div className="flex items-center gap-2">
              <select
                className="rounded border border-border bg-background px-2 py-1.5 text-sm"
                value={channel === "all" ? "email" : channel}
                onChange={(e) => setChannel(e.target.value)}
              >
                {CHANNELS.filter((c) => c.id !== "all").map((c) => (
                  <option key={c.id} value={c.id}>{c.label}</option>
                ))}
              </select>
              <span className="text-xs text-muted-foreground">
                {selectedCandidate ? `To: ${selectedCandidate.name}` : "Select a candidate"}
              </span>
            </div>
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder="Write a message…"
              rows={3}
            />
            <Button
              type="button"
              onClick={send}
              disabled={sending || !selectedCandidateId || !text || channel === "all"}
            >
              {sending ? <Spinner className="mr-2" /> : <Send className="mr-2 h-4 w-4" />}
              Send
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
