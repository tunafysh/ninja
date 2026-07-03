"use client";

import { check } from "@tauri-apps/plugin-updater";
import { RefreshCcw, X } from "lucide-react";
import { useCallback, useEffect } from "react";
import { useConfig } from "@/hooks/config";
import type { UpdateInfo } from "@/lib/types";
import { Button } from "./button";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from "./field";
import RegistryModal from "./registry-modal";
import { Switch } from "./switch";

async function checkForUpdates(
  setUpdateInfo: (v: UpdateInfo) => void,
  setShowUpdateDialog: (v: boolean) => void,
) {
  try {
    const update = await check();

    if (update) {
      setUpdateInfo({
        version: update.version,
        date: update.date,
        downloadAndInstall: update.downloadAndInstall,
        body: update.body,
      });
      setShowUpdateDialog(true);
    }
  } catch (err) {
    console.error("Update check failed", err);
  }
}
export default function SettingsDialog({
  platform: _platform,
  close,
  setUpdateInfo,
  setShowUpdateDialog,
}: {
  platform: string;
  close: () => void;
  setUpdateInfo: (v: UpdateInfo) => void;
  setShowUpdateDialog: (v: boolean) => void;
}) {
  const { config, setDevMode, setUpdates, saveConfig, fetchConfig } =
    useConfig();

  const stableSave = useCallback(() => saveConfig(), [saveConfig]);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  useEffect(() => {
    if (!config) return;

    const timeout = setTimeout(() => {
      stableSave();
    }, 1500);

    return () => clearTimeout(timeout);
  }, [config, stableSave]);

  return (
    <div
      className={`absolute inset-0 z-40 flex items-center justify-center bg-black/40`}
    >
      <div className="w-full h-full rounded-xl border bg-background shadow-xl py-6 flex flex-col items-center gap-5">
        <div className="w-full flex justify-between h-fit">
          <div></div>
          <h2 className="text-3xl font-semibold">Settings</h2>
          <Button
            variant="ghost"
            className="text-foreground/50"
            onClick={() => close()}
          >
            <X />
          </Button>
        </div>

        <FieldGroup className="w-full max-w-sm">
          <FieldLabel htmlFor="check-updates">
            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle>Enable updates</FieldTitle>
                <FieldDescription>
                  Enable checking for updates and installing them.
                </FieldDescription>
              </FieldContent>
              {config?.checkUpdates && (
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => {
                    checkForUpdates(setUpdateInfo, setShowUpdateDialog);
                  }}
                >
                  <RefreshCcw />
                </Button>
              )}
              <Switch
                id="check-updates"
                checked={config?.checkUpdates}
                onCheckedChange={(v) => setUpdates(v)}
              />
            </Field>
          </FieldLabel>
          <FieldLabel htmlFor="developer-mode">
            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle>Enable developer mode</FieldTitle>
                <FieldDescription>
                  By enabling developer mode you gain access to the Developer
                  tab where you can execute any DSL command.
                </FieldDescription>
              </FieldContent>
              <Switch
                id="developer-mode"
                checked={config?.devMode}
                onCheckedChange={(v) => setDevMode(v)}
              />
            </Field>
          </FieldLabel>
        </FieldGroup>

        <RegistryModal />
      </div>
    </div>
  );
}
