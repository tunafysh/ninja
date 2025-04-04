"use client"
import ApacheEditor from "@/components/apache-editor";
import { ApplicationMenubar } from "@/components/application-menubar";
import { ShadcnSidebar } from "@/components/swappable-sidebar";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import Image from "next/image";
import { useEffect, useState } from "react";
import { getPanelElement } from "react-resizable-panels";

export default function Home() {
        return (
            <div style={{ height: "calc(100vh - 48px)", borderRadius: '7px'}}>
            <ApplicationMenubar />
                <div className="flex flex-row h-full">
                    <ShadcnSidebar />
                    <ApacheEditor />
                </div>
          </div>
        );
}
