import { DialogTitle } from "@radix-ui/react-dialog";
import { Dialog, DialogContent, DialogFooter, DialogHeader } from "./dialog";
import { Button } from "./button";
import { Input } from "./input";
import { ReactNode, useEffect, useRef, useState } from "react";

type ArrayEditorProps = {
  title: string;
  /**
   * Value may be:
   *  - string[]                 -> array mode
   *  - [string,string][]        -> map mode (second element is description)
   */
  value?: string[] | [string, string][];
  /** auto: detect from `value`, array: treat as array of strings, map: treat as array of pairs */
  mode?: "auto" | "array" | "map";
  /** control dialog visibility (optional) */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** called when the working copy changes (immediate) */
  onChange?: (items: string[] | [string, string][]) => void;
  /** called when the user clicks Save (parent should persist) */
  onSave?: () => void;
  placeholder?: string;
  renderItem?: (item: string, index: number) => ReactNode;
};

type ArrayItem = { id: string; value: string };
type MapItem = { id: string; key: string; desc: string };

const uid = () => Math.random().toString(36).slice(2, 9);

/**
 * ArrayEditor
 *
 * A dialog for editing arrays or maps:
 * - Array mode: edits a `string[]` (each item is a single value)
 * - Map mode: edits `[string, string][]` (each item is [key, description])
 *
 * Key behaviors:
 * - Local editing with `onChange` updates while editing.
 * - Parent persists on `onSave` (Save button calls `onSave` then closes dialog).
 * - Reordering via native HTML drag-and-drop.
 * - Uses project `Button` and `Input` components and Tailwind classes.
 *
 * This file contains only the component and docs (no example usage).
 */
export default function ArrayEditor({
  title,
  value = [],
  mode = "auto",
  onChange,
  placeholder = "New item",
  renderItem,
  open,
  onOpenChange,
  onSave,
}: ArrayEditorProps) {
  const detectedMode: "array" | "map" =
    mode === "auto"
      ? (Array.isArray(value) && value.length > 0 && Array.isArray(value[0]) ? "map" : "array")
      : (mode as "array" | "map");

  const [arrayItems, setArrayItems] = useState<ArrayItem[]>(
    detectedMode === "array" ? (Array.isArray(value) ? (value as string[]).map((v) => ({ id: uid(), value: v })) : []) : []
  );

  const [mapItems, setMapItems] = useState<MapItem[]>(
    detectedMode === "map"
      ? (Array.isArray(value) ? (value as [string, string][]).map(([k, d]) => ({ id: uid(), key: k, desc: d })) : [])
      : []
  );

  const [newArrayValue, setNewArrayValue] = useState("");
  const [newKey, setNewKey] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const dragIndex = useRef<number | null>(null);

  useEffect(() => {
    if (detectedMode === "array") {
      setArrayItems(Array.isArray(value) ? (value as string[]).map((v) => ({ id: uid(), value: v })) : []);
    } else {
      setMapItems(Array.isArray(value) ? (value as [string, string][]).map(([k, d]) => ({ id: uid(), key: k, desc: d })) : []);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, detectedMode]);

  // Reset working copy when the dialog is opened so stale edits aren't shown
  useEffect(() => {
    if (!open) return;
    if (detectedMode === "array") {
      setArrayItems(Array.isArray(value) ? (value as string[]).map((v) => ({ id: uid(), value: v })) : []);
    } else {
      setMapItems(Array.isArray(value) ? (value as [string, string][]).map(([k, d]) => ({ id: uid(), key: k, desc: d })) : []);
    }
    setNewArrayValue("");
    setNewKey("");
    setNewDesc("");
    dragIndex.current = null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (detectedMode === "array") {
      onChange?.(arrayItems.map((i) => i.value));
    } else {
      onChange?.(mapItems.map((i): [string, string] => [i.key, i.desc]));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [arrayItems, mapItems, detectedMode]);

  function addArrayItem(v: string) {
    const t = v.trim();
    if (!t) return;
    setArrayItems((s) => [...s, { id: uid(), value: t }]);
    setNewArrayValue("");
  }

  function addMapItem(k: string, d: string) {
    const key = k.trim();
    const desc = d.trim();
    if (!key) return;
    setMapItems((s) => [...s, { id: uid(), key, desc }]);
    setNewKey("");
    setNewDesc("");
  }

  function removeArrayAt(i: number) {
    setArrayItems((s) => s.filter((_, idx) => idx !== i));
  }
  function removeMapAt(i: number) {
    setMapItems((s) => s.filter((_, idx) => idx !== i));
  }

  function onDragStart(e: React.DragEvent, index: number) {
    dragIndex.current = index;
    e.dataTransfer.effectAllowed = "move";
    try {
      e.dataTransfer.setData("text/plain", String(index));
    } catch {}
  }

  function onDragOver(e: React.DragEvent) {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  }

  function onDropArray(e: React.DragEvent, index: number) {
    e.preventDefault();
    const from = dragIndex.current;
    if (from == null || from === index) return;
    setArrayItems((s) => {
      const copy = s.slice();
      const [moved] = copy.splice(from, 1);
      copy.splice(index, 0, moved);
      return copy;
    });
    dragIndex.current = null;
  }

  function onDropMap(e: React.DragEvent, index: number) {
    e.preventDefault();
    const from = dragIndex.current;
    if (from == null || from === index) return;
    setMapItems((s) => {
      const copy = s.slice();
      const [moved] = copy.splice(from, 1);
      copy.splice(index, 0, moved);
      return copy;
    });
    dragIndex.current = null;
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-lg font-medium">{title}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
        {detectedMode === "array" ? (
          <>
            <div className="flex gap-2">
              <Input
                value={newArrayValue}
                onChange={(e: any) => setNewArrayValue(e.target.value)}
                onKeyDown={(e: React.KeyboardEvent) => {
                  if (e.key === "Enter") addArrayItem(newArrayValue);
                }}
                placeholder={placeholder}
                className="flex-1"
              />
              <Button onClick={() => addArrayItem(newArrayValue)}>Add</Button>
            </div>

            <div role="list" className="flex flex-col gap-2 max-h-80 overflow-auto">
              {arrayItems.map((it, idx) => (
                <div
                  key={it.id}
                  role="listitem"
                  draggable
                  onDragStart={(e) => onDragStart(e, idx)}
                  onDragEnd={() => {
                    dragIndex.current = null;
                  }}
                  onDragOver={onDragOver}
                  onDrop={(e) => onDropArray(e, idx)}
                  className="flex items-center gap-3 p-2 border rounded bg-white dark:bg-slate-800"
                >
                  <div className="cursor-grab text-sm text-slate-500">☰</div>

                  <div className="flex-1">
                    {renderItem ? (
                      renderItem(it.value, idx)
                    ) : (
                      <input
                        className="w-full bg-transparent outline-none"
                        value={it.value}
                        onChange={(e) =>
                          setArrayItems((s) => s.map((x, i) => (i === idx ? { ...x, value: e.target.value } : x)))
                        }
                      />
                    )}
                  </div>

                  <Button variant="ghost" onClick={() => removeArrayAt(idx)} className="text-sm px-2 py-1">
                    Remove
                  </Button>
                </div>
              ))}
            </div>
          </>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2">
              <Input value={newKey} onChange={(e: any) => setNewKey(e.target.value)} placeholder="Key" />
              <Input
                value={newDesc}
                onChange={(e: any) => setNewDesc(e.target.value)}
                placeholder="Description"
                onKeyDown={(e: React.KeyboardEvent) => {
                  if (e.key === "Enter") addMapItem(newKey, newDesc);
                }}
              />
            </div>
            <div className="flex justify-end">
              <Button onClick={() => addMapItem(newKey, newDesc)}>Add</Button>
            </div>

            <div role="list" className="flex flex-col gap-2 max-h-80 overflow-auto">
              {mapItems.map((it, idx) => (
                <div
                  key={it.id}
                  role="listitem"
                  draggable
                  onDragStart={(e) => onDragStart(e, idx)}
                  onDragEnd={() => {
                    dragIndex.current = null;
                  }}
                  onDragOver={onDragOver}
                  onDrop={(e) => onDropMap(e, idx)}
                  className="flex items-center gap-3 p-2 border rounded bg-white dark:bg-slate-800"
                >
                  <div className="cursor-grab text-sm text-slate-500">☰</div>

                  <div className="flex-1 grid grid-cols-2 gap-2">
                    <input
                      value={it.key}
                      onChange={(e) => setMapItems((s) => s.map((x, i) => (i === idx ? { ...x, key: e.target.value } : x)))}
                      className="w-full bg-transparent outline-none"
                    />
                    <input
                      value={it.desc}
                      onChange={(e) => setMapItems((s) => s.map((x, i) => (i === idx ? { ...x, desc: e.target.value } : x)))}
                      className="w-full bg-transparent outline-none"
                    />
                  </div>

                  <Button variant="ghost" onClick={() => removeMapAt(idx)} className="text-sm px-2 py-1">
                    Remove
                  </Button>
                </div>
              ))}
            </div>
          </>
        )}
      </div>

      <DialogFooter>
          <div className="flex gap-2">
            <Button
              onClick={() => {
                const payload = detectedMode === "array" ? arrayItems.map((i) => i.value) : mapItems.map((i): [string, string] => [i.key, i.desc]);
                onChange?.(payload as string[] | [string, string][]);
                onSave?.();
                onOpenChange?.(false);
              }}
            >
              Save
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}