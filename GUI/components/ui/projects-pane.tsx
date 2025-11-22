import { useState, useEffect } from "react";
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "@/components/ui/card";
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
}: {
  projects: string[];
  refreshProjects: () => void;
  openProjectsFolder: () => void;
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
    <Card className="bg-background border-none py-0 mt-4">
      <CardHeader className="p-3 md:p-4 pb-0 md:pb-2">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-base md:text-lg">Local Projects</CardTitle>
            <CardDescription className="text-xs md:text-sm">
              Your web projects in htdocs directory
            </CardDescription>
          </div>
          <div className="flex gap-3">
            <Button size="icon" onClick={refreshProjects}>
              <RefreshCcw />
            </Button>
            <Button size="icon" variant="outline" onClick={openProjectsFolder}>
              <FolderOpen />
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-0 md:p-0 flex">
        {/* Sidebar */}
        <div className="w-40 md:w-56 border-r bg-card/30 p-2">
          <ScrollArea className="h-full">
            <div className="space-y-1">
              {projects && projects.length > 0 ? (
                projects.map((p, i) => (
                  <button
                    key={i}
                    onClick={() => setSelected(selected === p ? null : p)}
                    className={`w-full text-left px-3 py-2 rounded-md text-sm transition
                      ${selected === p ? "bg-accent" : "hover:bg-accent/50"}`}
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
        <div className="flex-1 p-4 overflow-auto">
          {selected ? (
            <motion.div
              key={selected}
              initial={{ opacity: 0, y: 5 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.15 }}
              className="space-y-4"
            >
              <h2 className="text-lg font-semibold">{selected}</h2>
              <p className="text-sm text-muted-foreground">Project details and quick actions.</p>

              <div className="prose max-w-none text-sm">
                {loading ? (
                  <p className="text-muted-foreground">Loading README...</p>
                ) : readme ? (
                  <ReactMarkdown>{readme}</ReactMarkdown>
                ) : (
                  <p className="text-muted-foreground">No README found.</p>
                )}
              </div>

              <div className="flex gap-3">
                <Button variant="outline" size="sm" className="h-8">
                  <FileCode className="h-4 w-4 mr-1" /> Files
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
