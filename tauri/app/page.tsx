"use client"
import ApacheEditor from "@/components/apache-editor";
import { ApplicationMenubar } from "@/components/application-menubar";
import { SwappableSidebar } from "@/components/swappable-sidebar";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import Image from "next/image";
import { useEffect, useState } from "react";
import { getPanelElement } from "react-resizable-panels";

export default function Home() {
    const [collapsed, setCollapsed] = useState(false)
    const sidebar = getPanelElement('sidebar')
    
    useEffect(()=> {
        if(sidebar){
            if(collapsed){
                sidebar.style.width = "64px"
            }
            else {
                sidebar.style.width = "256px"
            }
        }
    }, [sidebar, collapsed])

        return (
            <div style={{ height: "calc(100vh - 48px)"}}>
            <ApplicationMenubar />
                <ResizablePanelGroup direction="horizontal" className="flex flex-row h-full">
                    <ResizablePanel collapsible collapsedSize={12} maxSize={27} minSize={12} defaultSize={27} id="sidebar">
                        <SwappableSidebar isCollapsed={collapsed} setIsCollapsed={setCollapsed} />
                    </ResizablePanel>
                    <ResizableHandle/>
                    <ResizablePanel id="editor">
                        <ApacheEditor />
                    </ResizablePanel>
                </ResizablePanelGroup>

          </div>
        );
}
