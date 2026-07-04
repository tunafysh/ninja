"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { RefreshCcw, Search, FolderOpen, Package, Sparkle } from "lucide-react";

import ArmoryCard from "@/components/ui/armory-card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import InstallCard from "@/components/ui/install-card";
import { ArmoryItem, ArmoryMetadata, Registry } from "@/lib/types";
import { useConfig } from "@/hooks/config";

export default function Armory({
  platform,
}: {
  platform: "mac" | "windows" | "linux" | "unknown";
}) {
  const { config } = useConfig();
  const [path, setPath] = useState("");
  const [shurikens, setShurikens] = useState<ArmoryItem[]>([]);
  const [installedShurikens, setInstalledShurikens] = useState<ArmoryItem[]>(
    [],
  );
  const [localShuriken, setLocalShuriken] = useState<ArmoryMetadata | null>(
    null,
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  // Helper function to fetch registries and map their items to include a 'source' property
  const fetchAndProcessShurikens = async () => {
    // 1. Invoke the updated command that returns a Record/HashMap of registries
    const registriesMap = await invoke<Record<string, Registry>>(
      "registry_get_all_registries",
    );

    const allItems: ArmoryItem[] = [];

    // 2. Iterate over the registries, inject the computed source reference into each item
    Object.entries(registriesMap).forEach(([registryName, registry]) => {
      if (registry && registry.shurikens) {
        registry.shurikens.forEach((item) => {
          // Identify the unique item string (fallback to name if id isn't populated)
          const itemId = item.id || item.name?.toLowerCase();

          allItems.push({
            ...item,
            // INJECTING SOURCE: Emits exactly what ShurikenReference::parse expects ("registry:id")
            source: `${registryName}:${itemId}`,
          });
        });
      }
    });

    setShurikens(allItems);
    setInstalledShurikens(allItems.filter((item) => item.installed));
  };

  const installLocalFile = async () => {
    const file = await open({
      filters: [{ name: "Shurikens", extensions: ["shuriken"] }],
    });

    if (!file) return;

    try {
      setPath(file);
      const res = await invoke<ArmoryMetadata>("open_shuriken", { path: file });
      setLocalShuriken(res);
    } catch (e) {
      console.error("Failed to open shuriken:", e);
    }
  };

  const refreshShurikens = async () => {
    setIsRefreshing(true);
    try {
      await fetchAndProcessShurikens();
    } catch (e) {
      console.error("Failed to refresh shurikens:", e);
    } finally {
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    setIsLoading(true);
    fetchAndProcessShurikens()
      .catch((e) => console.error("Initial fetch failed:", e))
      .finally(() => setIsLoading(false));
  }, []);

  const filteredShurikens = shurikens.filter(
    (s) =>
      s.name?.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.description?.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  return (
    <div className="relative w-full flex justify-center">
      <div className="h-full w-5/6 max-w-7xl">
        {/* Header */}
        <div className="pt-10 pb-8">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-3">
              <div className="relative">
                <svg className="w-8 h-8 absolute">
                  <defs>
                    <linearGradient
                      id="zapStrokeGradient"
                      x1="0%"
                      y1="0%"
                      x2="100%"
                      y2="100%"
                    >
                      <stop offset="0%" stopColor="#f97316" />
                      <stop offset="100%" stopColor="#a855f7" />
                    </linearGradient>
                  </defs>
                </svg>
                <Sparkle
                  className="mr-1 h-8 w-8"
                  style={{
                    fill: "none",
                    stroke: "url(#zapStrokeGradient)",
                    strokeWidth: 2,
                  }}
                />
              </div>
              <div>
                <h1 className="text-2xl font-bold text-white">Armory</h1>
                <p className="text-sm text-neutral-400">
                  Manage your Shurikens
                </p>
              </div>
            </div>
            <Button
              onClick={refreshShurikens}
              disabled={isRefreshing}
              variant="outline"
              className="gap-2"
            >
              <RefreshCcw
                className={`w-4 h-4 ${isRefreshing ? "animate-spin" : ""}`}
              />
              {isRefreshing ? "Refreshing..." : "Refresh"}
            </Button>
          </div>
        </div>

        {/* Installed Shurikens Section */}
        {installedShurikens.length > 0 && (
          <div className="mb-16">
            <div className="flex items-center gap-2 mb-6">
              <div className="p-1.5 rounded bg-green-500/20">
                <Package className="w-4 h-4 text-primary" />
              </div>
              <h2 className="text-2xl font-bold text-white">
                Installed ({installedShurikens.length})
              </h2>
              <div className="flex-1 h-px bg-neutral-700"></div>
            </div>
            <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-4">
              {installedShurikens.map((shuriken) => (
                <ArmoryCard shuriken={shuriken} key={shuriken.name} />
              ))}
            </div>
          </div>
        )}

        {/* Explore & Install Section */}
        <div>
          <div className="flex items-center gap-2 mb-6">
            <div className="p-1.5 rounded">
              <Search className="w-4 h-4 text-primary" />
            </div>
            <h2 className="text-xl font-bold text-white">
              Explore Shurikens ({filteredShurikens.length})
            </h2>
            <div className="flex-1 h-px bg-neutral-700"></div>
          </div>

          {/* Search and Install Bar */}
          <div className="flex gap-3 mb-8">
            <div className="flex-1 relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500 w-4 h-4 pointer-events-none" />
              <Input
                className="pl-10 bg-neutral-800 border-neutral-700 text-white placeholder:text-neutral-500 focus:border-blue-500"
                placeholder="Search by name or description..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <Button
              onClick={installLocalFile}
              className="gap-2 bg-emerald-600 hover:bg-emerald-700"
            >
              <FolderOpen className="w-4 h-4" />
              Install Local File
            </Button>
          </div>

          {/* Local Shuriken Install Card */}
          {/* Local Shuriken Install Card */}
          {localShuriken && (
            <InstallCard
              shuriken={localShuriken}
              path={path}
              onClose={() => setLocalShuriken(null)}
              onComplete={fetchAndProcessShurikens} // <-- Forces immediate local list update!
            />
          )}

          {/* Shurikens Grid */}
          {isLoading ? (
            <div className="flex items-center justify-center py-20">
              <div className="text-center">
                <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
                <p className="mt-4 text-neutral-400">Loading shurikens...</p>
              </div>
            </div>
          ) : filteredShurikens.length > 0 ? (
            <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-4">
              {filteredShurikens.map((shuriken) => (
                <ArmoryCard shuriken={shuriken} key={shuriken.name} />
              ))}
            </div>
          ) : (
            <div className="flex items-center justify-center py-20">
              <div className="text-center">
                <Package className="w-12 h-12 text-neutral-600 mx-auto mb-4" />
                <p className="text-neutral-400">
                  {searchQuery
                    ? "No shurikens found matching your search"
                    : "No shurikens available"}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
