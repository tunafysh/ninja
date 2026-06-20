"use client";

import { useState } from "react";
import { motion } from "motion/react";
import { ArmoryItem } from "@/lib/types";
import { Card } from "./card";
import ArmoryModal from "./armory-modal";

export default function ArmoryCard({ shuriken }: { shuriken: ArmoryItem }) {
  const [activeShuriken, setActiveShuriken] = useState<ArmoryItem | null>(null);

  return (
    <>
      <ArmoryModal
        shuriken={activeShuriken}
        onClose={() => setActiveShuriken(null)}
      />

      <motion.div
        onClick={() => setActiveShuriken(shuriken)}
        className="cursor-pointer transition hover:scale-[1.015]"
        whileHover={{ opacity: 0.9 }}
        whileTap={{ scale: 0.98 }}
      >
        <Card className="flex h-32 flex-col items-start justify-start gap-1 rounded-xl border bg-white p-4 shadow-sm dark:bg-neutral-900">
          <div className="w-full">
            <h2 className="text-lg font-semibold text-neutral-900 dark:text-neutral-100">
              {shuriken.name}
            </h2>
            <p className="line-clamp-2 text-sm text-neutral-600 dark:text-neutral-400">
              {shuriken.synopsis || shuriken.description}
            </p>
          </div>
        </Card>
      </motion.div>
    </>
  );
}
