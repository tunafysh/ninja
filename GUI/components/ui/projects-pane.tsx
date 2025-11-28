import { useState, useEffect } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { RefreshCcw, FolderOpen, FileCode } from "lucide-react";
import { motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";

export default function LocalProjectsSidebar({
  projects,
  refreshProjects,
  openProjectsFolder,
  openSpecificProject,
}: {
  projects: string[];
  refreshProjects: () => void;
  openProjectsFolder: () => void;
  openSpecificProject: (projectName: string) => void;
}) {
  const [selected, setSelected] = useState<string | null>(null);
  const [readme, setReadme] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!selected) return;

    setLoading(true);
    setReadme(null);

    invoke<string>("get_project_readme", { name: selected })
      .then((content) => setReadme(content))
      .catch((err) => setReadme(`Error: ${err}`))
      .finally(() => setLoading(false));
  }, [selected]);

  return (
    <Card className="bg-background border-none py-0 mt-4 shadow-sm rounded-xl">
      <CardHeader className="p-4 pb-2">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-lg font-semibold">Local Projects</CardTitle>
            <CardDescription className="text-sm">
              Your web projects in the <b>htdocs</b> directory
            </CardDescription>
          </div>

          <div className="flex gap-2">
            <Button
              size="icon"
              variant="ghost"
              className="hover:bg-accent rounded-lg"
              onClick={refreshProjects}
            >
              <RefreshCcw className="h-4 w-4" />
            </Button>

            <Button
              size="icon"
              variant="outline"
              className="rounded-lg"
              onClick={openProjectsFolder}
            >
              <FolderOpen className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-0 flex border-2 border-muted rounded-lg">
        {/* Sidebar */}
        <div className="w-52 border-r bg-muted/30 p-2 min-h-48">
          <ScrollArea className="h-full">
            <div className="space-y-1">
              {projects?.length ? (
                projects.map((p) => (
                  <button
                    key={p}
                    onClick={() => setSelected(selected === p ? null : p)}
                    className={`w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition
                      ${
                        selected === p
                          ? "bg-accent text-accent-foreground shadow-sm"
                          : "hover:bg-accent/40"
                      }`}
                  >
                    {p}
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

        {/* Details Pane */}
        <div className="flex-1 p-6 overflow-auto">
          {selected ? (
            <motion.div
              key={selected}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.18 }}
              className="space-y-5"
            >
              <div>
                <h2 className="text-xl font-semibold">{selected}</h2>
                <p className="text-sm text-muted-foreground">
                  Project details and quick actions
                </p>
              </div>

              <div className="prose prose-sm dark:prose-invert max-w-none bg-card rounded-lg p-4 border">
                {loading ? (
                  <p className="text-muted-foreground">Loading README...</p>
                ) : readme ? (
                  <ReactMarkdown>{readme}</ReactMarkdown>
                ) : (
                  <p className="text-muted-foreground">No README found.</p>
                )}
              </div>

              <div className="flex gap-2">
                <Button variant="outline" size="sm" className="h-8 rounded-lg" onClick={() => openSpecificProject(selected)}>
                  <FileCode className="h-4 w-4 mr-2" /> Open Files
                </Button>
              </div>
            </motion.div>
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
              Select a project to view details.
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
