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
  Check,
  BadgeCheck,
  Code2,
  ShieldCheck,
  Download,
} from "lucide-react";

export default function ArmoryCard({ shuriken }: { shuriken: ArmoryItem }) {
  const [active, setActive] = useState<ArmoryItem | boolean | null>(null);
  const id = useId();
  const ref = useRef<HTMLDivElement>(null);

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
              <div className="flex justify-between items-start mb-4">
                <div>
                  <h2 className="text-2xl font-bold text-neutral-800 dark:text-neutral-100">
                    {active.name}
                  </h2>
                  <p className="text-neutral-600 dark:text-neutral-400">
                    {active.synopsis}
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  <a
                    href={active.repository}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 px-4 py-2 text-sm font-medium bg-muted hover:bg-muted/70 rounded-md transition text-neutral-800 dark:text-neutral-100"
                  >
                    <ExternalLink className="w-4 h-4" />
                    Repo
                  </a>
                  <Button
                    onClick={() => alert(`Installing ${active.name}...`)}
                    disabled={shuriken.installed}
                  >
                    {shuriken.installed ? (
                      <>
                        <Check className="w-4 h-4" />
                        <p>Installed</p>
                      </>
                    ) : (
                      <>
                        <Download className="w-4 h-4" />
                        <p>Install</p>
                      </>
                    )}
                  </Button>
                </div>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">Description</h3>
                  <p className="text-sm text-neutral-700 dark:text-neutral-300 leading-relaxed">
                    {active.description || "No description provided."}
                  </p>
                </section>

                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">Details</h3>
                  <ul className="text-sm space-y-1 text-neutral-700 dark:text-neutral-300">
                    <li className="flex items-center gap-2"><Package className="w-4 h-4" /> <span>Version: {active.version}</span></li>
                    <li className="flex items-center gap-2"><ShieldCheck className="w-4 h-4" /> <span>License: {active.license}</span></li>
                    <li className="flex items-center gap-2"><BadgeCheck className="w-4 h-4" /> <span>Authors: {active.authors.join(", ")}</span></li>
                    <li className="flex items-center gap-2"><Code2 className="w-4 h-4" /> <span>Checksum: {active.checksum}</span></li>
                  </ul>
                </section>

                <section className="col-span-full">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground mb-1">Dependencies</h3>
                  <div className="flex flex-wrap gap-2">
                    {active.dependencies.length > 0 ? active.dependencies.map((dep) => (
                      <span
                        key={dep}
                        className="inline-block px-2 py-1 text-xs rounded bg-muted text-muted-foreground"
                      >
                        {dep}
                      </span>
                    )) : <p className="text-sm text-neutral-500">None</p>}
                  </div>
                </section>

                <section className="col-span-full">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground mb-1">Supported Platforms</h3>
                  <div className="flex flex-wrap gap-2">
                    {active.platforms.length > 0 ? active.platforms.map((plat) => (
                      <span
                        key={plat}
                        className="inline-block px-2 py-1 text-xs rounded bg-muted text-muted-foreground"
                      >
                        {plat}
                      </span>
                    )) : <p className="text-sm text-neutral-500">Unknown</p>}
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
        <Card className="h-40 p-4 flex flex-col justify-center items-start gap-2 rounded-xl border shadow-sm bg-white dark:bg-neutral-900">
          <div className="w-full">
            <h2 className="text-lg font-semibold text-neutral-900 dark:text-neutral-100">
              {shuriken.name}
            </h2>
            <p className="text-sm text-neutral-600 dark:text-neutral-400 line-clamp-3">
              {shuriken.synopsis}
            </p>
          </div>
        </Card>
      </motion.div>
    </>
  );
}