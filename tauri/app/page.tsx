"use client"
import { ApplicationMenubar } from "@/components/application-menubar";
import { SwappableSidebar } from "@/components/swappable-sidebar";
import Image from "next/image";

export default function Home() {
    return (
        <div className="h-screen">
            <ApplicationMenubar/>
            <SwappableSidebar/>
            
        </div>
    );
}
