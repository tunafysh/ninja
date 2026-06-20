import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { ArmoryItem } from "./types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function capitalizeFirstLetter(str: string) {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

export type InstallMethod = "url" | "registry" | "path";

export function resolveInstallSource(
  item: ArmoryItem,
  method: InstallMethod,
  customInput: string
) {
  const trimmedInput = customInput.trim();
  if (trimmedInput) return trimmedInput;

  switch (method) {
    case "url":
      return "url" in item ? item.url : "";

    case "registry":
      return "registry" in item
        ? `${item.registry}:${item.shuriken}`
        : "";

    case "path":
      return "path" in item
        ? item.path
        : "";

    default:
      return "";
  }
}
