"use client";

import React, { useEffect, useId, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { useOutsideClick } from "@/hooks/use-outside-click";
import { ArmoryItem } from "@/lib/types";
import { Card } from "./card";
import { Button } from "./button";
import {
  ExternalLink,
  Package,
  BadgeCheck,
  Code2,
  ShieldCheck,
  Download,
  Trash,
} from "lucide-react";
import { useShuriken } from "@/hooks/use-shuriken";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

const installShuriken = async (
  url: string,
  onSuccess?: () => void,
  onError?: (error: Error) => void
) => {
  try {
    // Pass URL directly - manager.install() auto-detects it's a URL
    await invoke("install_shuriken", { name: url });
    onSuccess?.();
  } catch (e) {
    const error =
      e instanceof Error ? e : new Error(String(e));
    onError?.(error);
  }
};

export default function ArmoryCard({ shuriken }: { shuriken: ArmoryItem }) {
  const { removeShuriken } = useShuriken();
  const [active, setActive] = useState<ArmoryItem | boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installMethod, setInstallMethod] = useState<"url" | "registry" | "path">(
    "registry"
  );
  const [customInput, setCustomInput] = useState("");
  const id = useId();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Set initial install method based on sourceType
    if (active && typeof active === "object") {
      const sourceType = active.sourceType || "registry";
      // Normalize 'file' to 'path'
      setInstallMethod(sourceType === "file" ? "path" : sourceType);
    }
  }, [active]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setActive(false);
    };

    document.body.style.overflow = active ? "hidden" : "auto";
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active]);

  useOutsideClick(ref, () => setActive(null));

  return (
    <>
      {/* Overlay */}
      <AnimatePresence>
        {active && typeof active === "object" && (
          <motion.div
            className="fixed inset-0 bg-black/50 z-40"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          />
        )}
      </AnimatePresence>

      {/* Modal */}
      <AnimatePresence>
        {active && typeof active === "object" && (
          <motion.div
            className="fixed inset-0 z-50 grid place-items-center p-4 overflow-y-auto"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            <div
              ref={ref}
              className="w-full max-w-4xl rounded-2xl bg-white dark:bg-neutral-900 shadow-xl p-6 relative"
            >
              {error && (
                <div className="mb-4 p-3 rounded-md bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm">
                  {error}
                </div>
              )}
              <div className="flex justify-between items-start mb-4">
                <div>
                  <h2 className="text-2xl font-bold text-neutral-800 dark:text-neutral-100">
                    {active.name}
                  </h2>
                  <p className="text-neutral-600 dark:text-neutral-400">
                    {active.synopsis || active.description}
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  <a
                    href={active.repository || "#"}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 px-4 py-2 text-sm font-medium bg-muted hover:bg-muted/70 rounded-md transition text-neutral-800 dark:text-neutral-100"
                  >
                    <ExternalLink className="w-4 h-4" />
                    Repo
                  </a>
                  {shuriken.installed ? (
                    <Button
                      variant={"destructive"}
                      onClick={() => removeShuriken(shuriken.name)}
                    >
                      <Trash className="w-4 h-4" />
                      <p>Remove</p>
                    </Button>
                  ) : (
                    <Button
                      disabled={installing}
                      onClick={() => {
                        setInstalling(true);
                        setError(null);
                        // Determine install source based on method
                        let installSource = "";
                        if (installMethod === "url" && "url" in active && active.url) {
                          installSource = active.url;
                        } else if (installMethod === "registry" && "registry" in active && "shuriken" in active) {
                          installSource = customInput || `${active.registry}:${active.shuriken}`;
                        } else if (installMethod === "path" && "path" in active) {
                          installSource = customInput || active.path;
                        } else {
                          installSource = customInput;
                        }
                        installShuriken(
                          installSource,
                          () => {
                            setInstalling(false);
                            setActive(null);
                            setCustomInput("");
                            setInstallMethod("registry");
                          },
                          (error) => {
                            setInstalling(false);
                            setError(error.message);
                          }
                        );
                      }}
                    >
                      <Download className="w-4 h-4" />
                      <p>{installing ? "Installing..." : "Install"}</p>
                    </Button>
                  )}
                </div>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Description
                  </h3>
                  <p className="text-sm text-neutral-700 dark:text-neutral-300 leading-relaxed">
                    {active.description || "No description provided."}
                  </p>
                </section>

                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Details
                  </h3>
                  <ul className="text-sm space-y-1 text-neutral-700 dark:text-neutral-300">
                    <li className="flex items-center gap-2">
                      <Package className="w-4 h-4" />
                      <span>Version: {active.version}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <ShieldCheck className="w-4 h-4" />
                      <span>License: {active.license}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <BadgeCheck className="w-4 h-4" />
                      <span>
                        Author: {active.author || active.authors?.join(", ")}
                      </span>
                    </li>
                    <li className="flex items-center gap-2">
                      <Code2 className="w-4 h-4" />
                      <span>Checksum: {active.checksum || "N/A"}</span>
                    </li>
                  </ul>
                </section>

                <section className="col-span-full">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground mb-3">
                    Installation Method
                  </h3>
                  <div className="space-y-3">
                    <div className="flex gap-2">
                      <button
                        onClick={() => {
                          setInstallMethod("url");
                          setCustomInput("");
                        }}
                        className={`px-3 py-1 text-xs rounded transition ${
                          installMethod === "url"
                            ? "bg-primary text-primary-foreground"
                            : "bg-muted text-muted-foreground hover:bg-muted/70"
                        }`}
                      >
                        Direct URL
                      </button>
                      <button
                        onClick={() => {
                          setInstallMethod("registry");
                          setCustomInput("");
                        }}
                        className={`px-3 py-1 text-xs rounded transition ${
                          installMethod === "registry"
                            ? "bg-primary text-primary-foreground"
                            : "bg-muted text-muted-foreground hover:bg-muted/70"
                        }`}
                      >
                        Registry Ref
                      </button>
                      <button
                        onClick={() => {
                          setInstallMethod("path");
                          setCustomInput("");
                        }}
                        className={`px-3 py-1 text-xs rounded transition ${
                          installMethod === "path"
                            ? "bg-primary text-primary-foreground"
                            : "bg-muted text-muted-foreground hover:bg-muted/70"
                        }`}
                      >
                        File Path
                      </button>
                    </div>

                    {installMethod === "url" && "url" in active && (
                      <div className="p-2 bg-muted/50 rounded text-xs text-muted-foreground break-all">
                        {active.url}
                      </div>
                    )}

                    {installMethod === "registry" && "registry" in active && "shuriken" in active && (
                      <input
                        type="text"
                        placeholder="e.g., ninja:caddy"
                        value={
                          customInput || `${active.registry}:${active.shuriken}`
                        }
                        onChange={(e) => setCustomInput(e.target.value)}
                        className="w-full px-2 py-1 text-xs rounded border border-muted-foreground bg-transparent"
                      />
                    )}

                    {installMethod === "path" && "path" in active && (
                      <input
                        type="text"
                        placeholder="/path/to/file.shuriken"
                        value={
                          customInput || active.path
                        }
                        onChange={(e) => setCustomInput(e.target.value)}
                        className="w-full px-2 py-1 text-xs rounded border border-muted-foreground bg-transparent"
                      />
                    )}
                  </div>
                </section>

                <section className="col-span-full">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground mb-1">
                    Supported Platforms
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    {active.platforms.length > 0 ? (
                      active.platforms.map((plat) => (
                        <span
                          key={plat}
                          className="inline-block px-2 py-1 text-xs rounded bg-muted text-muted-foreground"
                        >
                          {plat}
                        </span>
                      ))
                    ) : (
                      <p className="text-sm text-neutral-500">Unknown</p>
                    )}
                  </div>
                </section>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Card */}
      <motion.div
        onClick={() => setActive(shuriken)}
        className="cursor-pointer transition hover:scale-[1.015]"
        whileHover={{ opacity: 0.9 }}
        whileTap={{ scale: 0.98 }}
      >
        <Card className="h-32 p-4 flex flex-col justify-start items-start gap-1 rounded-xl border shadow-sm bg-white dark:bg-neutral-900">
          <div className="w-full">
            <h2 className="text-lg font-semibold text-neutral-900 dark:text-neutral-100">
              {shuriken.name}
            </h2>
            <p className="text-sm text-neutral-600 dark:text-neutral-400 line-clamp-2">
              {shuriken.synopsis || shuriken.description}
            </p>
          </div>
        </Card>
      </motion.div>
    </>
  );
}
