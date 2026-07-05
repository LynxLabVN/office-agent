import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Sparkles, Wand2 } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";

const SCRIPT_SECTIONS = [
  { key: "hook", label: "Hook" },
  { key: "problem", label: "Problem" },
  { key: "agitate", label: "Agitate" },
  { key: "solution", label: "Solution" },
  { key: "product", label: "Product" },
  { key: "proof", label: "Proof" },
  { key: "offer", label: "Offer" },
  { key: "cta", label: "CTA" },
  { key: "captions", label: "Captions" },
];

export default function ScriptWorkspacePage() {
  const { pieceId } = useParams<{ pieceId: string }>();
  const [sections, setSections] = useState<Record<string, string>>({});
  const [hooks, setHooks] = useState<string[]>([]);
  const [productSku, setProductSku] = useState("");
  const [format, setFormat] = useState("IG");
  const [saving, setSaving] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [loading, setLoading] = useState(true);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle(`Script · ${pieceId ?? "new"}`);
    setSubTitle("9-section script editor with AI hook generator.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle, pieceId]);

  useEffect(() => {
    if (!pieceId) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    api.marketing
      .getScript(pieceId)
      .then((res) => {
        setSections(res.sections ?? {});
        setHooks(res.hooks ?? []);
      })
      .catch(() => {
        setSections(Object.fromEntries(SCRIPT_SECTIONS.map((s) => [s.key, ""])));
      })
      .finally(() => setLoading(false));
  }, [pieceId]);

  const save = async () => {
    if (!pieceId) return;
    setSaving(true);
    try {
      await api.marketing.saveScript(pieceId, { sections, hooks });
      showToast("Script saved", "success");
    } catch (err) {
      showToast("Save failed" + ": " + String(err), "error");
    } finally {
      setSaving(false);
    }
  };

  const suggestHooks = async () => {
    if (!productSku) return;
    setGenerating(true);
    try {
      const res = await api.marketing.suggestHooks(productSku, format);
      setHooks((prev) => [...new Set([...prev, ...res.hooks])]);
      showToast(`${res.hooks.length} hooks generated`, "success");
    } catch (err) {
      showToast("Hook generation failed" + ": " + String(err), "error");
    } finally {
      setGenerating(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <Spinner className="text-2xl text-primary" />
      </div>
    );
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
      <div className="flex flex-col gap-4">
        {SCRIPT_SECTIONS.map(({ key, label }) => (
          <Card key={key}>
            <CardHeader>
              <CardTitle className="text-sm font-semibold uppercase tracking-wide">{label}</CardTitle>
            </CardHeader>
            <CardContent>
              <textarea
                value={sections[key] ?? ""}
                onChange={(e) => setSections((s) => ({ ...s, [key]: e.target.value }))}
                placeholder={`Write the ${label.toLowerCase()} section…`}
                rows={4}
              />
            </CardContent>
          </Card>
        ))}
        <div className="flex justify-end gap-2">
          <Button type="button" outlined onClick={save} disabled={saving}>
            {saving ? <Spinner className="mr-2" /> : null}
            Save script
          </Button>
        </div>
      </div>

      <div className="flex flex-col gap-4">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Wand2 className="h-4 w-4" /> Hook generator
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <div>
              <Label className="text-xs">Product SKU</Label>
              <Input
                value={productSku}
                onChange={(e) => setProductSku(e.target.value)}
                placeholder="e.g. SPF-50"
              />
            </div>
            <div>
              <Label className="text-xs">Format</Label>
              <select
                className="w-full rounded border border-border bg-background px-2 py-1.5 text-sm"
                value={format}
                onChange={(e) => setFormat(e.target.value)}
              >
                <option>YT</option>
                <option>IG</option>
                <option>FB</option>
                <option>TikTok</option>
              </select>
            </div>
            <Button type="button" onClick={suggestHooks} disabled={generating || !productSku}>
              {generating ? <Spinner className="mr-2" /> : <Sparkles className="mr-2 h-4 w-4" />}
              Generate hooks
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Suggested hooks</CardTitle>
          </CardHeader>
          <CardContent>
            {hooks.length === 0 ? (
              <p className="text-sm text-muted-foreground">No hooks yet.</p>
            ) : (
              <ul className="flex flex-col gap-2">
                {hooks.map((hook, idx) => (
                  <li
                    key={idx}
                    className="cursor-pointer rounded border border-border p-2 text-sm hover:bg-secondary"
                    onClick={() => setSections((s) => ({ ...s, hook: s.hook ? `${s.hook}\n${hook}` : hook }))}
                  >
                    {hook}
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
