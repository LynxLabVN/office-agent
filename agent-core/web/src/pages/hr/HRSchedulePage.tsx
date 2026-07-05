import { useEffect, useState } from "react";
import { Calendar, Clock, Monitor, Send } from "lucide-react";
import { api } from "@/lib/api";
import type { HRScheduleSlot } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { cn } from "@/lib/utils";

const EVENT_TYPES = [
  { id: "initial", label: "Initial screen" },
  { id: "technical", label: "Technical interview" },
  { id: "final", label: "Final interview" },
];

interface Booking {
  id: string;
  candidateId: string;
  candidateName: string;
  eventType: string;
  start: string;
  bookingUrl?: string;
}

export default function HRSchedulePage() {
  const [eventTypeId, setEventTypeId] = useState("initial");
  const [date, setDate] = useState(new Date().toISOString().split("T")[0]);
  const [slots, setSlots] = useState<HRScheduleSlot[]>([]);
  const [loading, setLoading] = useState(false);
  const [candidateId, setCandidateId] = useState("");
  const [candidateName, setCandidateName] = useState("");
  const [selectedSlot, setSelectedSlot] = useState<HRScheduleSlot | null>(null);
  const [bookings, setBookings] = useState<Booking[]>([]);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("Schedule");
    setSubTitle("Pick interview slots and send invites.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  const loadSlots = async () => {
    if (!eventTypeId || !date) return;
    setLoading(true);
    try {
      const res = await api.hr.getScheduleSlots(eventTypeId, date);
      setSlots(res.slots ?? []);
    } catch (err) {
      showToast("Load failed" + ": " + String(err), "error");
    } finally {
      setLoading(false);
    }
  };

  const book = async () => {
    if (!eventTypeId || !selectedSlot || !candidateId) return;
    try {
      const res = await api.hr.bookSchedule(eventTypeId, selectedSlot.start, candidateId);
      showToast("Booked" + " " + (res.booking_url ?? ""), "success");
      setBookings((prev) => [
        {
          id: Math.random().toString(36).slice(2),
          candidateId,
          candidateName: candidateName || candidateId,
          eventType: EVENT_TYPES.find((e) => e.id === eventTypeId)?.label ?? eventTypeId,
          start: selectedSlot.start,
          bookingUrl: res.booking_url,
        },
        ...prev,
      ]);
      setSelectedSlot(null);
    } catch (err) {
      showToast("Booking failed" + ": " + String(err), "error");
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 lg:grid-cols-[1fr_320px]">
        <div className="flex flex-col gap-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <Monitor className="h-4 w-4" /> Interviewer calendar
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex aspect-video items-center justify-center rounded border border-dashed border-border bg-secondary/20 text-sm text-muted-foreground">
                Cal.com embed area
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <Calendar className="h-4 w-4" /> Slot picker
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="sm:col-span-2">
                  <Label className="text-xs">Event type</Label>
                  <select
                    className="w-full rounded border border-border bg-background px-2 py-1.5 text-sm"
                    value={eventTypeId}
                    onChange={(e) => setEventTypeId(e.target.value)}
                  >
                    {EVENT_TYPES.map((e) => (
                      <option key={e.id} value={e.id}>{e.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <Label className="text-xs">Date</Label>
                  <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
                </div>
              </div>
              <Button type="button" onClick={loadSlots} disabled={loading || !eventTypeId || !date}>
                {loading ? <Spinner className="mr-2" /> : <Clock className="mr-2 h-4 w-4" />}
                List slots
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Available slots</CardTitle>
            </CardHeader>
            <CardContent>
              {slots.length === 0 ? (
                <p className="text-sm text-muted-foreground">No slots loaded.</p>
              ) : (
                <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                  {slots.map((slot, idx) => (
                    <button
                      key={idx}
                      onClick={() => setSelectedSlot(slot)}
                      disabled={!slot.available}
                      className={cn(
                        "rounded border p-3 text-left text-sm transition-colors",
                        selectedSlot === slot
                          ? "border-midground bg-midground/10 text-midground"
                          : slot.available
                          ? "border-border bg-card hover:bg-secondary"
                          : "border-border/50 bg-secondary/30 text-muted-foreground",
                      )}
                    >
                      <div className="font-medium">
                        {new Date(slot.start).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {new Date(slot.end).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        <div className="flex flex-col gap-4">
          <Card className="h-fit">
            <CardHeader>
              <CardTitle className="text-sm">Book interview</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
              <div>
                <Label className="text-xs">Candidate ID</Label>
                <Input value={candidateId} onChange={(e) => setCandidateId(e.target.value)} placeholder="candidate-id" />
              </div>
              <div>
                <Label className="text-xs">Candidate name</Label>
                <Input value={candidateName} onChange={(e) => setCandidateName(e.target.value)} placeholder="Name" />
              </div>
              {selectedSlot && (
                <div className="rounded bg-secondary/30 p-2 text-xs text-muted-foreground">
                  Selected: {new Date(selectedSlot.start).toLocaleString()}
                </div>
              )}
              <Button type="button" onClick={book} disabled={!selectedSlot || !candidateId || !eventTypeId}>
                <Send className="mr-2 h-4 w-4" />
                Book & send invite
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Upcoming interviews</CardTitle>
        </CardHeader>
        <CardContent>
          {bookings.length === 0 ? (
            <p className="text-sm text-muted-foreground">No upcoming interviews.</p>
          ) : (
            <div className="flex flex-col gap-2">
              {bookings.map((b) => (
                <div key={b.id} className="flex items-center justify-between rounded border border-border p-3 text-sm">
                  <div>
                    <div className="font-medium">{b.candidateName}</div>
                    <div className="text-xs text-muted-foreground">{b.eventType} · {new Date(b.start).toLocaleString()}</div>
                  </div>
                  {b.bookingUrl && (
                    <a href={b.bookingUrl} target="_blank" rel="noreferrer" className="text-xs text-midground hover:underline">
                      Open booking
                    </a>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
