import { useState } from "react"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import ReactMarkdown from "react-markdown"
import { Upload, Loader2 } from "lucide-react"
import { downloadUpdate } from "@/lib/updater"
import { UpdateInfo } from "@/lib/types"

interface Props {
  open: boolean
  onOpenChange: (v: boolean) => void
  updateInfo: UpdateInfo
}

export default function UpdateDialog({
  open,
  onOpenChange,
  updateInfo
}: Props) {
  const [progress, setProgress] = useState<number | null>(null)
  const isDownloading = progress !== null

  if (!updateInfo) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md p-0 overflow-hidden rounded-xl shadow-xl">
        <div className="flex flex-col h-105">

          {/* Top */}
          <div className="flex-4 flex flex-col items-center justify-center bg-linear-to-br from-indigo-500 via-purple-500 to-pink-500">
            <Upload className="h-16 w-16 mb-4 text-white drop-shadow-lg" />
          </div>

          {/* Bottom */}
          <div className="flex-6 flex items-center justify-center bg-background">
            <div className="w-full max-w-sm p-8 text-center flex flex-col gap-4">
              <h2 className="text-2xl font-bold">
                {isDownloading ? "Downloading update…" : "An update is available!"}
              </h2>

              {updateInfo.body && (
                <div className="text-sm text-muted-foreground text-left">
                  <ReactMarkdown>{updateInfo.body}</ReactMarkdown>
                </div>
              )}

              <Button
                disabled={isDownloading}
                className="relative w-full py-3 rounded-lg overflow-hidden"
                style={
                  progress !== null
                    ? {
                        background: `linear-gradient(
                          to right,
                          hsl(var(--primary)) ${progress}%,
                          hsl(var(--primary) / 0.2) ${progress}%
                        )`
                      }
                    : undefined
                }
                onClick={async () => {
                  setProgress(0)
                  await downloadUpdate(setProgress)
                }}
              >
                <span className="relative z-10 flex items-center gap-2">
                  {isDownloading && <Loader2 className="h-4 w-4 animate-spin" />}
                  {isDownloading
                    ? progress === 100
                      ? "Installing…"
                      : `Downloading… ${progress}%`
                    : `Download v${updateInfo.version}`}
                </span>
              </Button>
            </div>
          </div>

        </div>
      </DialogContent>
    </Dialog>
  )
}
