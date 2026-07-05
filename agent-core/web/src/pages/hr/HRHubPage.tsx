import { useEffect } from "react";
import { Link } from "react-router-dom";
import {
  BarChart3,
  Briefcase,
  Calendar,
  MessageSquare,
  Users,
  Workflow,
} from "lucide-react";
import { usePageHeader } from "@/contexts/usePageHeader";

const HR_CARDS = [
  {
    to: "/hr/jobs",
    icon: Briefcase,
    title: "Jobs",
    description: "Job descriptions, posting, and board console.",
    hint: "JD editor",
  },
  {
    to: "/hr/pipeline",
    icon: Workflow,
    title: "Pipeline",
    description: "Applied → Screened → Interview → Offer → Hired/Rejected.",
    hint: "kanban",
  },
  {
    to: "/hr/candidates",
    icon: Users,
    title: "Candidates",
    description: "CV viewer, score breakdown, and comparison.",
    hint: "screening",
  },
  {
    to: "/hr/schedule",
    icon: Calendar,
    title: "Schedule",
    description: "Interview slot picker and invite sender.",
    hint: "Cal.com",
  },
  {
    to: "/hr/comms",
    icon: MessageSquare,
    title: "Comms",
    description: "Candidate threads by channel with templates.",
    hint: "Zalo / Telegram / Email",
  },
  {
    to: "/hr/analytics",
    icon: BarChart3,
    title: "Analytics",
    description: "Time-to-hire, funnel, and source effectiveness.",
    hint: "reports",
  },
];

export default function HRHubPage() {
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle("HR");
    setSubTitle("Hiring pipeline, candidates, scheduling, and reports.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle]);

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {HR_CARDS.map((card) => (
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
    </div>
  );
}
