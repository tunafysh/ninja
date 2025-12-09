import { ArmoryMetadata } from '@/lib/types';
import { motion, AnimatePresence } from 'motion/react';
import {
  Package,
  BadgeCheck,
  ShieldCheck,
  Download,
  Check,
} from "lucide-react";
import { Spinner } from "@/components/ui/spinner";
import { useState, useRef, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { useOutsideClick } from '@/hooks/use-outside-click';
import { invoke } from '@tauri-apps/api/core';

type InstallCardProps = {
  shuriken: ArmoryMetadata;
  path: string;
  onClose: () => void;
};

export default function InstallCard({ shuriken, path, onClose }: InstallCardProps) {
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);

  const ref = useRef<HTMLDivElement>(null);

  // Lock scroll + close on Escape
  useEffect(() => {
    document.body.style.overflow = "hidden";

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = "auto";
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);

  // Close on outside click
  useOutsideClick(ref, onClose);

  const handleInstall = async () => {
    try {
      setInstalling(true);
      setInstalled(false);
      await invoke("install_shuriken", { path });
      setInstalled(true);
    } catch (err) {
      console.error("Failed to install shuriken:", err);
      setInstalled(false);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <>
      {/* Overlay */}
      <AnimatePresence>
        <motion.div
          className="fixed inset-0 bg-black/50 z-40"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
        />
      </AnimatePresence>

      {/* Modal */}
      <AnimatePresence>
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
                  {shuriken.name}
                </h2>
                <p className="text-neutral-600 dark:text-neutral-400">
                  {shuriken.synopsis}
                </p>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  onClick={handleInstall}
                  disabled={installing}
                >
                  {installing ? (
                    <Spinner className="w-4 h-4" />
                  ) : installed ? (
                    <Check className="w-4 h-4" />
                  ) : (
                    <Download className="w-4 h-4" />
                  )}
                  <p className="ml-1">
                    {installing
                      ? "Installing..."
                      : installed
                      ? "Installed"
                      : "Install"}
                  </p>
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
                    <span>Version: {shuriken.version}</span>
                  </li>
                  <li className="flex items-center gap-2">
                    <ShieldCheck className="w-4 h-4" />
                    <span>License: {shuriken.license}</span>
                  </li>
                  <li className="flex items-center gap-2">
                    <BadgeCheck className="w-4 h-4" />
                    <span>Authors: {shuriken.authors?.join(", ")}</span>
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
      </AnimatePresence>
    </>
  );
}
