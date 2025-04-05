import { ApplicationMenubar } from "@/components/application-menubar";
import { ShadcnSidebar } from "@/components/swappable-sidebar";

export default function Home() {
        return (
            <div style={{ height: "calc(100vh - 48px)", borderRadius: '7px'}}>
            <ApplicationMenubar />
          </div>
        );
}
