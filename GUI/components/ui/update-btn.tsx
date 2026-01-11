import { useState } from "react"
import { Button } from "./button"

export default function UpdateBtn({ version }: { version: string}) {
    const [mode, setMode] = useState<"idle" | "downloading" | "installing" | "done" | "error">("idle")
    
    return (
        <Button variant={mode === "error"? "destructive": "default"}className="w-full py-3 rounded-lg">
            Download v{version}
        </Button>
    )
}