import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Download, Film, Music, Play, Plus, Trash2, Upload } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import { Card, CardContent, CardHeader, CardTitle } from "@nous-research/ui/ui/components/card";
import { Select, SelectOption } from "@nous-research/ui/ui/components/select";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";
import { HERMES_BASE_PATH } from "@/lib/api";

interface Shot {
  in: string;
  out: string;
  description: string;
  duration: string;
}

const MUSIC_TRACKS = ["Upbeat", "Corporate", "Lo-fi"];

function formatShot(shot: Shot): string {
  return `${shot.in} - ${shot.out} | ${shot.description} | ${shot.duration}`;
}

export default function EditPreviewPage() {
  const { pieceId } = useParams<{ pieceId: string }>();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [footagePath, setFootagePath] = useState("");
  const [selectedFileName, setSelectedFileName] = useState("");
  const [shots, setShots] = useState<Shot[]>([
    { in: "", out: "", description: "", duration: "" },
  ]);
  const [captions, setCaptions] = useState("");
  const [music, setMusic] = useState("");
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [rendering, setRendering] = useState(false);
  const { showToast } = useToast();
  const { setTitle, setSubTitle } = usePageHeader();

  useEffect(() => {
    setTitle(`Edit & Preview · ${pieceId ?? "new"}`);
    setSubTitle("Upload raw footage, build a shotlist, and render previews.");
    return () => {
      setTitle("");
      setSubTitle("");
    };
  }, [setTitle, setSubTitle, pieceId]);

  const ingest = async () => {
    if (!pieceId || !footagePath) return;
    setLoading(true);
    try {
      await api.marketing.ingestEdit(pieceId, footagePath);
      showToast("Footage ingested", "success");
    } catch (err) {
      showToast("Ingest failed" + ": " + String(err), "error");
    } finally {
      setLoading(false);
    }
  };

  const loadPreview = async () => {
    if (!pieceId) return;
    try {
      const res = await api.marketing.previewEdit(pieceId);
      if (res.ok) {
        const blob = await res.blob();
        setPreviewUrl(URL.createObjectURL(blob));
      }
    } catch (err) {
      showToast("Preview failed" + ": " + String(err), "error");
    }
  };

  const render = async () => {
    if (!pieceId) return;
    setRendering(true);
    try {
      const shotlist = shots.filter((s) => s.description || s.in || s.out || s.duration).map(formatShot);
      const res = await api.marketing.renderEdit(pieceId, shotlist, captions, music || undefined);
      if (res.preview_url) {
        setPreviewUrl(res.preview_url);
      } else {
        await loadPreview();
      }
      showToast("Render complete", "success");
    } catch (err) {
      showToast("Render failed" + ": " + String(err), "error");
    } finally {
      setRendering(false);
    }
  };

  const updateShot = (idx: number, field: keyof Shot, value: string) => {
    setShots((list) => list.map((s, i) => (i === idx ? { ...s, [field]: value } : s)));
  };

  const addShot = () => setShots((list) => [...list, { in: "", out: "", description: "", duration: "" }]);

  const removeShot = (idx: number) => {
    setShots((list) => (list.length > 1 ? list.filter((_, i) => i !== idx) : list));
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setSelectedFileName(file.name);
    setFootagePath(`/uploads/${file.name}`);
  };

  const downloadUrl = previewUrl ?? `${HERMES_BASE_PATH}/api/marketing/edit/download?piece_id=${encodeURIComponent(pieceId ?? "")}`;

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_420px]">
      <div className="flex flex-col gap-4">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Upload className="h-4 w-4" /> Raw footage
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <div
              onClick={() => fileInputRef.current?.click()}
              className="flex cursor-pointer flex-col items-center justify-center gap-2 rounded border border-dashed border-border bg-secondary/30 px-4 py-8 text-center text-muted-foreground transition-colors hover:bg-secondary/50"
            >
              <Upload className="h-6 w-6 opacity-40" />
              <p className="text-xs font-medium">
                {selectedFileName ? selectedFileName : "Drop raw footage here or click to browse"}
              </p>
              <p className="text-[10px] text-text-tertiary">MP4, MOV, MKV accepted</p>
            </div>
            <input
              ref={fileInputRef}
              type="file"
              accept="video/*"
              className="hidden"
              onChange={handleFileChange}
            />
            <Label className="text-xs">Footage path</Label>
            <Input
              value={footagePath}
              onChange={(e) => setFootagePath(e.target.value)}
              placeholder="/path/to/footage.mp4"
            />
            <Button type="button" onClick={ingest} disabled={loading || !footagePath}>
              {loading ? <Spinner className="mr-2" /> : <Upload className="mr-2 h-4 w-4" />}
              Ingest footage
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Film className="h-4 w-4" /> Shotlist
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <div className="overflow-x-auto rounded border border-border">
              <table className="w-full text-xs">
                <thead className="bg-secondary/50 text-left text-muted-foreground">
                  <tr>
                    <th className="px-3 py-2 font-medium">Shot #</th>
                    <th className="px-3 py-2 font-medium">In-point</th>
                    <th className="px-3 py-2 font-medium">Out-point</th>
                    <th className="px-3 py-2 font-medium">Description</th>
                    <th className="px-3 py-2 font-medium">Duration</th>
                    <th className="px-3 py-2 font-medium"></th>
                  </tr>
                </thead>
                <tbody>
                  {shots.map((shot, idx) => (
                    <tr key={idx} className="border-t border-border">
                      <td className="px-3 py-2 align-middle text-muted-foreground">{idx + 1}</td>
                      <td className="px-1 py-1 align-middle">
                        <Input
                          value={shot.in}
                          onChange={(e) => updateShot(idx, "in", e.target.value)}
                          placeholder="00:00:00"
                          className="h-8 min-w-[5rem] text-xs"
                        />
                      </td>
                      <td className="px-1 py-1 align-middle">
                        <Input
                          value={shot.out}
                          onChange={(e) => updateShot(idx, "out", e.target.value)}
                          placeholder="00:00:05"
                          className="h-8 min-w-[5rem] text-xs"
                        />
                      </td>
                      <td className="px-1 py-1 align-middle">
                        <Input
                          value={shot.description}
                          onChange={(e) => updateShot(idx, "description", e.target.value)}
                          placeholder="Shot description"
                          className="h-8 min-w-[10rem] text-xs"
                        />
                      </td>
                      <td className="px-1 py-1 align-middle">
                        <Input
                          value={shot.duration}
                          onChange={(e) => updateShot(idx, "duration", e.target.value)}
                          placeholder="5s"
                          className="h-8 min-w-[4rem] text-xs"
                        />
                      </td>
                      <td className="px-1 py-1 align-middle">
                        <Button
                          type="button"
                          ghost
                          size="icon"
                          onClick={() => removeShot(idx)}
                          disabled={shots.length === 1}
                          title="Remove shot"
                        >
                          <Trash2 className="h-3.5 w-3.5 text-muted-foreground" />
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <Button type="button" outlined onClick={addShot}>
              <Plus className="mr-2 h-4 w-4" /> Add shot
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Captions & music</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <Label className="text-xs">Captions</Label>
            <textarea
              value={captions}
              onChange={(e) => setCaptions(e.target.value)}
              placeholder="On-screen captions…"
              rows={4}
              className="w-full rounded border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-midground/30"
            />
            <Label className="flex items-center gap-2 text-xs">
              <Music className="h-3.5 w-3.5" /> Music track
            </Label>
            <Select
              value={music}
              onValueChange={setMusic}
              placeholder="Select a music track"
            >
              {MUSIC_TRACKS.map((track) => (
                <SelectOption key={track} value={track}>
                  {track}
                </SelectOption>
              ))}
            </Select>
          </CardContent>
        </Card>

        <Button
          type="button"
          onClick={render}
          disabled={rendering || shots.every((s) => !s.description && !s.in && !s.out && !s.duration)}
        >
          {rendering ? <Spinner className="mr-2" /> : <Play className="mr-2 h-4 w-4" />}
          {rendering ? "Rendering…" : "Render preview"}
        </Button>
      </div>

      <div>
        <Card className="sticky top-4">
          <CardHeader>
            <CardTitle className="text-sm">Preview</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            {previewUrl ? (
              <video
                src={previewUrl}
                controls
                className="w-full rounded"
                poster="/favicon.ico"
              />
            ) : (
              <div className="flex aspect-video flex-col items-center justify-center rounded border border-dashed border-border bg-secondary/30 text-muted-foreground">
                <Film className="mb-2 h-8 w-8 opacity-40" />
                <p className="text-xs">Render to see preview</p>
              </div>
            )}
            <a
              href={downloadUrl}
              download
              className="inline-flex items-center justify-center gap-2 rounded border border-border bg-card px-3 py-2 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary"
            >
              <Download className="h-3.5 w-3.5" /> Download MP4
            </a>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
