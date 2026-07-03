"use client";

import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import {
  BadgeCheck,
  Code2,
  Download,
  ExternalLink,
  Package,
  ShieldCheck,
  Trash,
} from "lucide-react";

import { useOutsideClick } from "@/hooks/use-outside-click";
import { useInstallShuriken } from "@/hooks/use-install-shuriken";
import { useShuriken } from "@/hooks/use-shuriken";
import { ArmoryItem } from "@/lib/types";
import { cn } from "@/lib/utils";

import { Button } from "./button";

type ArmoryModalProps = {
  shuriken: ArmoryItem | null;
  onClose: () => void;
};

function getInstallTarget(item: ArmoryItem): string {
  if ("registry" in item && item.registry) {
    return `${item.registry}:${item.shuriken}`;
  }

  if ("url" in item) {
    return item.url;
  }

  if ("path" in item) {
    return item.path;
  }

  return item.name;
}

export default function ArmoryModal({ shuriken, onClose }: ArmoryModalProps) {
  const ref = useRef<HTMLDivElement>(null);

  const { removeShuriken } = useShuriken();
  const { install, installing, progress, error } = useInstallShuriken();

  const [localError, setLocalError] = useState<string | null>(null);

  useOutsideClick(ref, onClose);

  useEffect(() => {
    if (!shuriken) return;

    setLocalError(null);

    document.body.style.overflow = "hidden";

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = "";
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [shuriken, onClose]);

  const handleInstall = async () => {
    if (!shuriken) return;

    setLocalError(null);

    if (await install(getInstallTarget(shuriken))) {
      onClose();
    }
  };

  if (!shuriken) {
    return null;
  }

  const displayError = localError || error;
  const installPercent = installing ? (progress?.progress ?? 0) : 0;
  const installLabel = installing
    ? (progress?.stage ?? "Installing...")
    : "Install";

  return (
    <AnimatePresence>
      <>
        <motion.div
          className="fixed inset-0 z-40 bg-black/50"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
        />

        <motion.div
          className="fixed inset-0 z-50 grid place-items-center overflow-y-auto p-4"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
        >
          <div
            ref={ref}
            className="relative w-full max-w-4xl rounded-xl bg-white p-6 shadow-xl dark:bg-neutral-900"
          >
            {displayError && (
              <div className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-600 dark:bg-red-900/20 dark:text-red-400">
                {displayError}
              </div>
            )}

            <div className="mb-4 flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
              <div>
                <h2 className="text-2xl font-bold">{shuriken.name}</h2>

                <p className="text-muted-foreground">
                  {shuriken.synopsis || shuriken.description}
                </p>
              </div>

              <div className="flex items-center gap-2">
                {shuriken.repository && (
                  <Button asChild variant="secondary">
                    <a
                      href={shuriken.repository}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="h-4 w-4" />
                      Repo
                    </a>
                  </Button>
                )}

                {shuriken.installed ? (
                  <Button
                    variant="destructive"
                    onClick={() => removeShuriken(shuriken.name)}
                  >
                    <Trash className="h-4 w-4" />
                    Remove
                  </Button>
                ) : (
                  <Button
                    disabled={installing}
                    onClick={handleInstall}
                    className={cn(
                      installing &&
                        "relative overflow-hidden bg-muted text-foreground disabled:opacity-100",
                    )}
                  >
                    {installing && (
                      <span
                        className="absolute inset-y-0 left-0 bg-primary transition-[width]"
                        style={{ width: `${installPercent}%` }}
                      />
                    )}

                    <span className="relative z-10 flex items-center gap-2">
                      <Download className="h-4 w-4" />
                      {installLabel}

                      {installing && (
                        <span className="tabular-nums">{installPercent}%</span>
                      )}
                    </span>
                  </Button>
                )}
              </div>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <section>
                <h3 className="mb-2 text-sm font-medium uppercase text-muted-foreground">
                  Description
                </h3>

                <p className="text-sm leading-relaxed">
                  {shuriken.description || "No description provided."}
                </p>
              </section>

              <section>
                <h3 className="mb-2 text-sm font-medium uppercase text-muted-foreground">
                  Details
                </h3>

                <ul className="space-y-2 text-sm">
                  <li className="flex items-center gap-2">
                    <Package className="h-4 w-4" />
                    Version: {shuriken.version || "N/A"}
                  </li>

                  <li className="flex items-center gap-2">
                    <ShieldCheck className="h-4 w-4" />
                    License: {shuriken.license || "N/A"}
                  </li>

                  <li className="flex items-center gap-2">
                    <BadgeCheck className="h-4 w-4" />
                    Author:{" "}
                    {shuriken.author ?? shuriken.authors?.join(", ") ?? "N/A"}
                  </li>

                  <li className="flex items-center gap-2">
                    <Code2 className="h-4 w-4" />
                    Checksum: {shuriken.checksum || "N/A"}
                  </li>
                </ul>
              </section>

              <section className="md:col-span-2">
                <h3 className="mb-2 text-sm font-medium uppercase text-muted-foreground">
                  Supported Platforms
                </h3>

                <div className="flex flex-wrap gap-2">
                  {shuriken.platforms.length ? (
                    shuriken.platforms.map((platform) => (
                      <span
                        key={platform}
                        className="rounded bg-muted px-2 py-1 text-xs text-muted-foreground"
                      >
                        {platform}
                      </span>
                    ))
                  ) : (
                    <span className="text-sm text-muted-foreground">
                      Unknown
                    </span>
                  )}
                </div>
              </section>
            </div>
          </div>
        </motion.div>
      </>
    </AnimatePresence>
  );
}
