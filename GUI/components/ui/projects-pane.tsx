import { useState, useEffect } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { RefreshCcw, FolderOpen, FileCode } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import { capitalizeFirstLetter } from "@/lib/utils";

interface LocalProjectsSidebarProps {
  projects: string[];
  refreshProjects: () => void;
  openProjectsFolder: () => void;
  openSpecificProject: (projectName: string) => void;
  gridView: "grid" | "list";
}

export default function LocalProjectsSidebar({
  projects,
  refreshProjects,
  openProjectsFolder,
  openSpecificProject,
  gridView,
}: LocalProjectsSidebarProps) {
  const [selected, setSelected] = useState<string | null>(null);
  const [readmeCache, setReadmeCache] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!projects?.length) return;

    const fetchReadmes = async () => {
      setLoading(true);
      const cache: Record<string, string> = {};
      await Promise.all(
        projects.map(async (p) => {
          try {
            const content = await invoke<string>("get_project_readme", { name: p });
            cache[p] = content;
          } catch (err) {
            cache[p] = `Error: ${err}`;
          }
        })
      );
      setReadmeCache(cache);
      setLoading(false);
    };

    fetchReadmes();
  }, [projects]);

  const toggleExpand = (project: string) => {
    setSelected((prev) => (prev === project ? null : project));
  };

  const renderReadme = (project: string) => {
    const readme = readmeCache[project] || "No README available.";
    return <ReactMarkdown>{readme}</ReactMarkdown>;
  };

  return (
    <Card className="bg-background border-none py-0 mt-4 shadow-sm rounded-xl">
      {/* Header */}
      <CardHeader className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-lg font-semibold">Local Projects</CardTitle>
            <CardDescription className="text-sm">
              Your web projects in the <b>htdocs</b> directory
            </CardDescription>
          </div>

          <div className="flex gap-2">
            <Button size="icon" variant="ghost" className="hover:bg-accent rounded-lg" onClick={refreshProjects}>
              <RefreshCcw className="h-4 w-4" />
            </Button>

            <Button size="icon" variant="outline" className="rounded-lg" onClick={openProjectsFolder}>
              <FolderOpen className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>

      {/* Grid View */}
      {gridView === "grid" ? (
        <CardContent>
          {projects?.length ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {projects.map((p) => {
                const isExpanded = selected === p;
                return (
                  <div
                    key={p}
                    className={`shadow-sm rounded-lg overflow-hidden cursor-pointer transition-all duration-300 p-1.5 ${
                      isExpanded ? "shadow-xl bg-card" : "bg-card hover:scale-105"
                    }`}
                    onClick={() => toggleExpand(p)}
                  >
                    <CardHeader className="flex flex-row items-center justify-between p-2">
                      <CardTitle className="text-sm font-medium">{capitalizeFirstLetter(p)}</CardTitle>
                      <Button
                        size="icon"
                        variant="outline"
                        onClick={(e) => {
                          e.stopPropagation();
                          openSpecificProject(p);
                        }}
                      >
                        <FolderOpen className="w-4 h-4" />
                      </Button>
                    </CardHeader>

                    <div
                      className={`overflow-hidden transition-all duration-300 ${
                        isExpanded ? "max-h-96 opacity-100 p-4 border-t" : "max-h-0 opacity-0"
                      }`}
                    >
                      {isExpanded && (loading && !readmeCache[p] ? (
                        <p className="text-muted-foreground">Loading README...</p>
                      ) : (
                        renderReadme(p)
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="text-center text-muted-foreground py-6 text-sm">
              No projects found.
            </div>
          )}
        </CardContent>
      ) : (
        // List View
        <CardContent className="p-0 flex border-2 border-muted rounded-lg">
          <div className="w-52 border-r bg-muted/30 p-2 min-h-48">
            <ScrollArea className="h-full">
              <div className="space-y-1">
                {projects?.length ? (
                  projects.map((p) => (
                    <button
                      key={p}
                      onClick={() => toggleExpand(p)}
                      className={`w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-all duration-300 ${
                        selected === p ? "bg-accent text-accent-foreground shadow-sm" : "hover:bg-accent/40"
                      }`}
                    >
                      {capitalizeFirstLetter(p)}
                    </button>
                  ))
                ) : (
                  <div className="text-center text-muted-foreground py-6 text-sm">
                    No projects found.
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>

          <div className="flex-1 p-6 overflow-auto transition-all duration-300">
            {selected ? (
              <div className="space-y-5">
                <h2 className="text-xl font-semibold">{selected}</h2>
                <div className="prose prose-sm dark:prose-invert max-w-none bg-card rounded-lg p-4 border transition-all duration-300">
                  {loading && !readmeCache[selected] ? (
                    <p className="text-muted-foreground">Loading README...</p>
                  ) : (
                    renderReadme(selected)
                  )}
                </div>
                <Button onClick={() => openSpecificProject(selected)}>
                  <FileCode className="h-4 w-4" />
                  Open file
                </Button>
              </div>
            ) : (
              <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
                Select a project to view details.
              </div>
            )}
          </div>
        </CardContent>
      )}
    </Card>
  );
}
