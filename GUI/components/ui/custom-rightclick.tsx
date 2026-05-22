import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { useConfig } from "@/hooks/config";
import { ContextMenuContent, ContextMenuItem } from "@radix-ui/react-context-menu";

export default async function CustomRightClick({ children }: { children: React.ReactNode}) {
    const { config } = useConfig();
    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>
                {children}
            </ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem>
                    Reload
                </ContextMenuItem>
                {config?.devMode &&
                (<ContextMenuItem>
                    Open DevTools
                </ContextMenuItem>)}
            </ContextMenuContent>
        </ContextMenu>
    )
}