"use client";

import { useEffect, useMemo, useRef, useState } from "react";
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
import { InstallMethod, cn, resolveInstallSource } from "@/lib/utils";
import { Button } from "./button";
import { Input } from "./input";

type ArmoryModalProps = {
  shuriken: ArmoryItem | null;
  onClose: () => void;
};

const getInitialInstallMethod = (item: ArmoryItem): InstallMethod => {
  // enforce priority order explicitly
  if ("registry" in item && item.registry) return "registry";
  if ("url" in item && item.url) return "url";
  if ("path" in item && item.path) return "path";

  // fallback if metadata lies or is incomplete
  if ("sourceType" in item && item.sourceType) {
    if (item.sourceType === "file") return "path";
    return item.sourceType as InstallMethod;
  }

  return "registry";
};
const getInstallPreview = (item: ArmoryItem, method: InstallMethod) => {
  switch (method) {
    case "url":
      return "url" in item ? item.url : "";
    case "registry":
      return "registry" in item ? `${item.registry}:${item.shuriken}` : "";
    case "path":
      return "path" in item ? item.path : "";
  }
};

export default function ArmoryModal({ shuriken, onClose }: ArmoryModalProps) {
  const ref = useRef<HTMLDivElement>(null);
  const { removeShuriken } = useShuriken();
  const { install, installing, progress, error } = useInstallShuriken();
  const [installMethod, setInstallMethod] = useState<InstallMethod>("registry");
  const [customInput, setCustomInput] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  useOutsideClick(ref, onClose);

  useEffect(() => {
    if (!shuriken) return;

    setInstallMethod(getInitialInstallMethod(shuriken));
    setCustomInput("");
    setLocalError(null);
  }, [shuriken]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    document.body.style.overflow = shuriken ? "hidden" : "auto";
    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = "auto";
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, shuriken]);

  const installPreview = useMemo(() => {
    return shuriken ? getInstallPreview(shuriken, installMethod) : "";
  }, [installMethod, shuriken]);

  const displayError = localError || error;
  const installPercent = installing ? progress?.progress ?? 0 : 0;
  const installLabel = installing
    ? progress?.stage || "Installing..."
    : "Install";

  const handleInstall = async () => {
    if (!shuriken) return;

    setLocalError(null);
    const source = resolveInstallSource(shuriken, installMethod, customInput);

    if (!source) {
      setLocalError("Choose a valid install source.");
      return;
    }

    const installed = await install(source);
    if (installed) onClose();
  };

  return (
    <AnimatePresence>
      {shuriken && (
        <>
          <motion.div
            className="fixed inset-0 z-40 bg-black/50"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          />

          <motion.div
            className="fixed inset-0 z-50 grid place-items-center overflow-y-auto p-4"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
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
                  <h2 className="text-2xl font-bold text-neutral-800 dark:text-neutral-100">
                    {shuriken.name}
                  </h2>
                  <p className="text-neutral-600 dark:text-neutral-400">
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
                          "relative overflow-hidden bg-muted text-foreground hover:bg-muted disabled:opacity-100"
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
                          <span className="tabular-nums">
                            {installPercent}%
                          </span>
                        )}
                      </span>
                    </Button>
                  )}
                </div>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Description
                  </h3>
                  <p className="text-sm leading-relaxed text-neutral-700 dark:text-neutral-300">
                    {shuriken.description || "No description provided."}
                  </p>
                </section>

                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Details
                  </h3>
                  <ul className="space-y-1 text-sm text-neutral-700 dark:text-neutral-300">
                    <li className="flex items-center gap-2">
                      <Package className="h-4 w-4" />
                      <span>Version: {shuriken.version || "N/A"}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <ShieldCheck className="h-4 w-4" />
                      <span>License: {shuriken.license || "N/A"}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <BadgeCheck className="h-4 w-4" />
                      <span>
                        Author:{" "}
                        {shuriken.author || shuriken.authors?.join(", ") || "N/A"}
                      </span>
                    </li>
                    <li className="flex items-center gap-2">
                      <Code2 className="h-4 w-4" />
                      <span>Checksum: {shuriken.checksum || "N/A"}</span>
                    </li>
                  </ul>
                </section>

                <section className="col-span-full">
                  <h3 className="mb-3 text-sm font-medium uppercase text-muted-foreground">
                    Installation Method
                  </h3>
                </section>

                <section className="col-span-full">
                  <h3 className="mb-1 text-sm font-medium uppercase text-muted-foreground">
                    Supported Platforms
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    {shuriken.platforms.length > 0 ? (
                      shuriken.platforms.map((platform) => (
                        <span
                          key={platform}
                          className="inline-block rounded bg-muted px-2 py-1 text-xs text-muted-foreground"
                        >
                          {platform}
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
        </>
      )}
    </AnimatePresence>
  );
}
