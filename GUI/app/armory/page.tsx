"use client"
import { ApplicationMenubar } from "@/components/ui/application-menubar";
import ArmoryCard from "@/components/ui/armory-card";
import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";
import { useEffect, useState } from "react";

export default function StorePage() {
    const [platform, setPlatform] = useState<"mac" | "windows" | "linux" | "unknown">("unknown")
    const [shurikens, setShurikens] = useState([
      {
        "name": "Apache HTTP Server",
        "label": "apache",
        "synopsis": "A powerful, flexible, and free web server.",
        "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
        "version": "1.0.0",
        "installed": false,
        "authors": ["Apache Software Foundation"],
        "license": "Apache-2.0",
        "repository": "https://github.com/apache/httpd",
        "dependencies": ["openssl", "zlib", "php"],
        "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
        "checksum": "sha256:abc123...",
      },
      {
        "name": "Apache HTTP Server",
        "label": "apache",
        "synopsis": "A powerful, flexible, and free web server.",
        "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
        "version": "1.0.0",
        "installed": false,
        "authors": ["Apache Software Foundation"],
        "license": "Apache-2.0",
        "repository": "https://github.com/apache/httpd",
        "dependencies": ["openssl", "zlib", "php"],
        "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
        "checksum": "sha256:abc123...",
      },
      {
        "name": "Apache HTTP Server",
        "label": "apache",
        "synopsis": "A powerful, flexible, and free web server.",
        "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
        "version": "1.0.0",
        "installed": false,
        "authors": ["Apache Software Foundation"],
        "license": "Apache-2.0",
        "repository": "https://github.com/apache/httpd",
        "dependencies": ["openssl", "zlib", "php"],
        "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
        "checksum": "sha256:abc123...",
      },
      {
        "name": "Apache HTTP Server",
        "label": "apache",
        "synopsis": "A powerful, flexible, and free web server.",
        "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
        "version": "1.0.0",
        "installed": false,
        "authors": ["Apache Software Foundation"],
        "license": "Apache-2.0",
        "repository": "https://github.com/apache/httpd",
        "dependencies": ["openssl", "zlib", "php"],
        "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
        "checksum": "sha256:abc123...",
      },
      {
        "name": "Apache HTTP Server",
        "label": "apache",
        "synopsis": "A powerful, flexible, and free web server.",
        "description": "Apache is the most widely used web server software. It is open-source and supports a variety of features including CGI, SSL, virtual domains, and more.",
        "version": "1.0.0",
        "installed": false,
        "authors": ["Apache Software Foundation"],
        "license": "Apache-2.0",
        "repository": "https://github.com/apache/httpd",
        "dependencies": ["openssl", "zlib", "php"],
        "platforms": ["x86_64-linux-gnu", "aarch64-windows-msvc", "aarch64-apple-darwin"],
        "checksum": "sha256:abc123...",
      }
    ])
    useEffect(() => {
        // Detect platform
        const userAgent = window.navigator.userAgent.toLowerCase()
        if (userAgent.indexOf("mac") !== -1) {
          setPlatform("mac")
        } else if (userAgent.indexOf("win") !== -1) {
          setPlatform("windows")
        } else if (userAgent.indexOf("linux") !== -1) {
          setPlatform("linux")
        }
    })

    return (
        <div className="relative w-screen h-screen overflow-hidden flex justify-center">
            <ApplicationMenubar platform={platform} activeWindow="Armory" />
            <div className={`h-full w-5/6 ${platform == "mac"? "mt-8": "mt-10"}`}>
              <div id="search" className="w-full flex justify-center items-center my-10">
                <div className="relative w-full">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 w-4 h-4 pointer-events-none" />
                  <Input
                    className="pl-10 w-full"
                    placeholder="Search..."
                  />
                </div>
              </div>
              <div className="w-full flex justify-center">
                <h1 className="font-bold text-2xl select-none">Browse for more shurikens</h1>
              </div>
              <div className="grid gap-4 grid-cols-4 mt-10">
                  {
                    shurikens.map((shuriken) => (
                      <ArmoryCard shuriken={shuriken} />
                    ))
                  }
              </div>
            </div>
        </div>
    )
}