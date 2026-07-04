"use client";

import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import {
  Package,
  BadgeCheck,
  ShieldCheck,
  Download,
  Check,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

import { Spinner } from "@/components/ui/spinner";
import { Button } from "@/components/ui/button";
import { useOutsideClick } from "@/hooks/use-outside-click";
import { ArmoryMetadata } from "@/lib/types";

type InstallCardProps = {
  shuriken: ArmoryMetadata | null; // Make this nullable so AnimatePresence works
  path: string;
  onClose: () => void;
  onComplete?: () => void; // Added to refresh dashboard items instantly
};

export default function InstallCard({
  shuriken,
  path,
  onClose,
  onComplete,
}: InstallCardProps) {
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Lock scroll + close on Escape
  useEffect(() => {
    if (!shuriken) return;

    document.body.style.overflow = "hidden";

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = "";
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [shuriken, onClose]);

  // Close on outside click
  useOutsideClick(ref, onClose);

  const handleInstall = async () => {
    try {
      setInstalling(true);
      setInstalled(false);
      await invoke("install_shuriken", { source: path });
      setInstalled(true);

      // Let the parent dashboard know it needs to refresh its list
      if (onComplete) {
        onComplete();
      }
    } catch (err) {
      console.error("Failed to install shuriken:", err);
      setInstalled(false);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <AnimatePresence>
      {shuriken && (
        <div className="fixed inset-0 z-40">
          {/* Overlay - Now cleanly animates out */}
          <motion.div
            className="fixed inset-0 bg-black/50"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            onClick={onClose}
          />

          {/* Modal Container */}
          <motion.div
            className="fixed inset-0 z-50 grid place-items-center p-4 overflow-y-auto"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.2 }}
          >
            <div
              ref={ref}
              className="w-full max-w-4xl rounded-2xl bg-white dark:bg-neutral-900 shadow-xl p-6 relative text-foreground"
            >
              <div className="flex justify-between items-start mb-4">
                <div>
                  <h2 className="text-2xl font-bold text-neutral-800 dark:text-neutral-100">
                    {shuriken.name}
                  </h2>
                  <p className="text-neutral-600 dark:text-neutral-400 mt-1">
                    {shuriken.synopsis}
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  <Button
                    onClick={handleInstall}
                    disabled={installing || installed}
                  >
                    {installing ? (
                      <Spinner className="w-4 h-4" />
                    ) : installed ? (
                      <Check className="w-4 h-4" />
                    ) : (
                      <Download className="w-4 h-4" />
                    )}
                    <span className="ml-2">
                      {installing
                        ? "Installing..."
                        : installed
                          ? "Installed"
                          : "Install"}
                    </span>
                  </Button>
                </div>
              </div>

              <div className="grid gap-6 md:grid-cols-2">
                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Description
                  </h3>
                  <p className="text-sm text-neutral-700 dark:text-neutral-300 leading-relaxed">
                    {shuriken.description || "No description provided."}
                  </p>
                </section>

                <section className="space-y-2">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground">
                    Details
                  </h3>
                  <ul className="text-sm space-y-1 text-neutral-700 dark:text-neutral-300">
                    <li className="flex items-center gap-2">
                      <Package className="w-4 h-4" />
                      <span>Version: {shuriken.version || "N/A"}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <ShieldCheck className="w-4 h-4" />
                      <span>License: {shuriken.license || "N/A"}</span>
                    </li>
                    <li className="flex items-center gap-2">
                      <BadgeCheck className="w-4 h-4" />
                      <span>
                        Authors: {shuriken.authors?.join(", ") || "N/A"}
                      </span>
                    </li>
                  </ul>
                </section>

                <section className="col-span-full">
                  <h3 className="text-sm font-medium uppercase text-muted-foreground mb-1">
                    Supported Platforms
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    <span className="inline-block px-2 py-1 text-xs rounded bg-muted text-muted-foreground">
                      {shuriken.platform}
                    </span>
                  </div>
                </section>
              </div>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
