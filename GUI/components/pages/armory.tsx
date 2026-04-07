"use client"
import { ApplicationMenubar } from "@/components/ui/application-menubar";
import ArmoryCard from "@/components/ui/armory-card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event"
import { RefreshCcw, Search } from "lucide-react";
import InstallCard from "@/components/ui/install-card";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { ArmoryItem, ArmoryMetadata } from "@/lib/types";
import ArrayEditor from "@/components/ui/array-editor";
import { useConfig } from "@/hooks/config";

export default function Armory({platform}: {platform: "mac" | "windows" | "linux" | "unknown"}) {
  const { config, addRegistry, removeRegistry, fetchConfig } = useConfig();
  const [registryEditorOpen, setRegistryEditorOpen] = useState(false);
  const [pendingRegistries, setPendingRegistries] = useState<[string, string][]>([]);
  const [path, setPath] = useState("");
  const [shurikens, setShurikens] = useState<ArmoryItem[]>([])
   const [localShuriken, setLocalShuriken] = useState<ArmoryMetadata | null>(null);

   const installLocalFile = async () => {
     const file = await open({
       filters: [
         {
           name: "Shurikens",
           extensions: ["shuriken"],
         },
       ],
     });
 
     if (!file) {
       console.log("No file selected");
       return;
     }
 
     console.log("Selected file:", file);
 
     try {
       // 👇 tell TypeScript what we expect back
       setPath(file)
       const res = await invoke<ArmoryMetadata>("open_shuriken", { path: file });
 
       console.log(res);
 
       // Option A: show it in a dedicated Install UI
       setLocalShuriken(res);
 
       // Option B (optional): also add it to the list
       // setShurikens(prev => [...prev, res]);
     } catch (e) {
       console.error("Failed to open shuriken:", e);
     }
   };

   useEffect(() => {
    invoke<ArmoryItem[]>("registry_get_all_shurikens")
      .then((e) => setShurikens(e))
   })

    return (
      <div className="relative w-screen h-screen overflow-hidden flex justify-center">
        <div className="h-full w-5/6">
          <div className="w-full flex justify-between items-center mt-10">
            <h1 className="font-bold text-2xl select-none">Explore shurikens</h1>
          </div>
          <div id="search" className="w-full flex justify-center items-center my-10">
            <div className="flex gap-2 w-full">
              <div className="relative w-full">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 w-4 h-4 pointer-events-none" />
                <Input className="pl-10 w-full" placeholder="Search..." />
              </div>
              <Button onClick={installLocalFile}>Install local file</Button>
            </div>
          </div>
  
          {/* 👇 show the InstallCard if we have a local shuriken */}
          {localShuriken && (
            <div className="mt-4">
              <InstallCard shuriken={localShuriken} path={path} onClose={() => setLocalShuriken(null)} />
            </div>
          )}
  
          <div className="grid gap-4 grid-cols-4 mt-10">
            {shurikens.map((shuriken) => (
              <ArmoryCard shuriken={shuriken} key={shuriken.name} />
            ))}
          </div>
        </div>
      </div>
    );
}