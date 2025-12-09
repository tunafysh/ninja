"use client"
import { ApplicationMenubar } from "@/components/ui/application-menubar";
import ArmoryCard from "@/components/ui/armory-card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event"
import { Search } from "lucide-react";
import InstallCard from "@/components/ui/install-card";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { ArmoryItem, ArmoryMetadata } from "@/lib/types";

export default function Armory({platform}: {platform: "mac" | "windows" | "linux" | "unknown"}) {
  const [path, setPath] = useState("");
  const [shurikens, setShurikens] = useState([
    {
      "name": "Apache HTTP",
      "label": "apache",
      "synopsis": "A powerful, flexible, and free web server.",
      "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
      "version": "1.0.0",
      "installed": true,
      "authors": ["Apache Software Foundation"],
      "license": "Apache-2.0",
      "repository": "https://github.com/apache/httpd",
      "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:abc123...",
    },
    {
      "name": "MariaDB",
      "label": "mysql",
      "synopsis": "A popular fork of MySQL Server.",
      "description": "MariaDB is a widely-used open-source relational database management system known for its reliability, performance, and ease of use. It supports SQL, replication, storage engines, and robust tooling.",
      "version": "1.0.0",
      "installed": true,
      "authors": ["Oracle Corporation"],
      "license": "GPL-2.0",
      "repository": "https://github.com/MariaDB/server",
      "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:def456..."
    },
    {
      "name": "PHP",
      "label": "php",
      "synopsis": "A fast, flexible, and pragmatic scripting language.",
      "description": "PHP is a widely-used general-purpose scripting language especially suited to web development. It powers many modern applications and provides rich extensions, FFI support, and strong ecosystem tooling.",
      "version": "1.0.0",
      "installed": false,
      "authors": ["The PHP Group"],
      "license": "PHP-3.01",
      "repository": "https://github.com/php/php-src",
      "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:ghi789..."
    },
    {
      "name": "Redis",
      "label": "redis",
      "synopsis": "An in-memory data store used as a database, cache, and message broker.",
      "description": "Redis is a high-performance in-memory key-value data store. It supports advanced data structures, pub/sub, clustering, persistence, and is widely used for caching and real-time systems.",
      "version": "1.0.0",
      "installed": false,
      "authors": ["Redis Ltd."],
      "license": "BSD-3-Clause",
      "repository": "https://github.com/redis/redis",
      "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:jkl012..."
    },
    {
      "name": "Nginx",
      "label": "nginx",
      "synopsis": "A high-performance web server and reverse proxy.",
      "description": "Nginx is a lightweight, high-performance web server known for its event-driven architecture, efficient resource usage, and reverse proxy capabilities. It excels at handling large numbers of concurrent connections.",
          "version": "1.0.0",
      "installed": false,
      "authors": ["NGINX Inc."],
      "license": "BSD-2-Clause",
      "repository": "https://github.com/nginx/nginx",
      "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:mno345..."
    },
    {
      "name": "PostgreSQL",
      "label": "postgres",
      "synopsis": "A powerful, open-source object-relational database.",
      "description": "PostgreSQL is a robust, standards-compliant, open-source relational database. It offers advanced features such as full-text search, extensibility, JSONB, window functions, and superior ACID compliance.",
      "version": "1.0.0",
      "installed": false,
      "authors": ["PostgreSQL Global Development Group"],
      "license": "PostgreSQL",
      "repository": "https://github.com/postgres/postgres",
      "platforms": ["x86_64-linux-gnu", "x86_64-windows-msvc", "aarch64-apple-darwin"],
      "checksum": "sha256:pqr678..."
    }
  ])
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

    return (
      <div className="relative w-screen h-screen overflow-hidden flex justify-center">
        <div className="h-full w-5/6">
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
  
          <div className="w-full flex justify-center">
            <h1 className="font-bold text-2xl select-none">Browse for more shurikens</h1>
          </div>
  
          <div className="grid gap-4 grid-cols-4 mt-10">
            {shurikens.map((shuriken) => (
              <ArmoryCard shuriken={shuriken} key={shuriken.name} />
            ))}
          </div>
        </div>
      </div>
    );
}