import { useEffect, useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import type { MarketingProduct } from "@/lib/api";
import { Button } from "@nous-research/ui/ui/components/button";
import { Input } from "@nous-research/ui/ui/components/input";
import { Label } from "@nous-research/ui/ui/components/label";
import { Spinner } from "@nous-research/ui/ui/components/spinner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@nous-research/ui/ui/components/card";
import { usePageHeader } from "@/contexts/usePageHeader";
import { useToast } from "@nous-research/ui/hooks/use-toast";

function emptyProduct(): MarketingProduct {
  return { sku: "", name: "", description: "", price: null, image_url: null };
}

export default function CatalogPage() {
  const [products, setProducts] = useState<MarketingProduct[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<MarketingProduct | null>(null);
  const [saving, setSaving] = useState(false);
  const { showToast } = useToast();
  const { setTitle, setSubTitle, setEnd } = usePageHeader();

  useEffect(() => {
    setTitle("Catalog");
    setSubTitle("Product list for scripts and campaigns.");
    setEnd(
      <Button type="button" size="sm" onClick={() => setEditing(emptyProduct())}>
        <Plus className="mr-2 h-4 w-4" />
        Add product
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
    api.marketing
      .getCatalog()
      .then((res) => setProducts(res.products ?? []))
      .catch((err) => showToast("Load failed" + ": " + String(err), "error"))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const save = async () => {
    if (!editing) return;
    setSaving(true);
    try {
      if (products.some((p) => p.sku === editing.sku)) {
        await api.marketing.updateProduct(editing.sku, editing);
        showToast("Product updated", "success");
      } else {
        await api.marketing.createProduct(editing);
        showToast("Product created", "success");
      }
      setEditing(null);
      load();
    } catch (err) {
      showToast("Save failed" + ": " + String(err), "error");
    } finally {
      setSaving(false);
    }
  };

  const remove = async (sku: string) => {
    try {
      await api.marketing.deleteProduct(sku);
      showToast("Product deleted", "success");
      load();
    } catch (err) {
      showToast("Delete failed" + ": " + String(err), "error");
    }
  };

  return (
    <div className="flex flex-col gap-4">
      {editing && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">{editing.sku ? "Edit product" : "New product"}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 sm:grid-cols-2">
            <div>
              <Label className="text-xs">SKU</Label>
              <Input
                value={editing.sku}
                onChange={(e) => setEditing((p) => ({ ...p!, sku: e.target.value }))}
                placeholder="SKU"
              />
            </div>
            <div>
              <Label className="text-xs">Name</Label>
              <Input
                value={editing.name}
                onChange={(e) => setEditing((p) => ({ ...p!, name: e.target.value }))}
                placeholder="Product name"
              />
            </div>
            <div>
              <Label className="text-xs">Price</Label>
              <Input
                type="number"
                value={editing.price ?? ""}
                onChange={(e) =>
                  setEditing((p) => ({ ...p!, price: e.target.value ? Number(e.target.value) : null }))
                }
                placeholder="Price"
              />
            </div>
            <div>
              <Label className="text-xs">Image URL</Label>
              <Input
                value={editing.image_url ?? ""}
                onChange={(e) =>
                  setEditing((p) => ({ ...p!, image_url: e.target.value || null }))
                }
                placeholder="https://…"
              />
            </div>
            <div className="sm:col-span-2">
              <Label className="text-xs">Description</Label>
              <Input
                value={editing.description}
                onChange={(e) => setEditing((p) => ({ ...p!, description: e.target.value }))}
                placeholder="Description"
              />
            </div>
            <div className="flex gap-2 sm:col-span-2">
              <Button type="button" onClick={save} disabled={saving}>
                {saving ? <Spinner className="mr-2" /> : null}
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
        <CardContent className="p-0">
          {loading ? (
            <div className="flex items-center justify-center py-24">
              <Spinner className="text-2xl text-primary" />
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-xs text-muted-foreground">
                  <th className="py-3 pl-4 text-left font-medium">SKU</th>
                  <th className="py-3 text-left font-medium">Name</th>
                  <th className="py-3 text-right font-medium">Price</th>
                  <th className="py-3 pr-4 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {products.map((p) => (
                  <tr key={p.sku} className="border-b border-border/50 hover:bg-secondary/20">
                    <td className="py-3 pl-4 font-mono-ui text-xs">{p.sku}</td>
                    <td className="py-3 pr-4">{p.name}</td>
                    <td className="py-3 pr-4 text-right">{p.price ?? "—"}</td>
                    <td className="py-3 pr-4 text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          type="button"
                          ghost
                          size="icon"
                          onClick={() => setEditing(p)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button type="button" ghost size="icon" onClick={() => remove(p.sku)}>
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
