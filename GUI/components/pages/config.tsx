"use client"

import { useState } from "react"
import { Input } from "../ui/input"
import { Button } from "../ui/button"
import { SaveIcon } from "lucide-react"
import { Item, ItemActions, ItemContent, ItemTitle } from "../ui/item"
import { useShuriken } from "@/hooks/use-shuriken"
import { invoke } from "@tauri-apps/api/core"

function capitalizeFirstLetter(str: string) {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

export default function Configuration() {
  const { allShurikens, refreshShurikens } = useShuriken()
  const [saving, setSaving] = useState<string | null>(null)

  const handleSave = async (shurikenName: string) => {
    try {
      setSaving(shurikenName)
      await invoke("configure_shuriken", { name: shurikenName })
      await refreshShurikens()
    } catch (err) {
      console.error("Failed to configure shuriken:", err)
    } finally {
      setSaving(null)
    }
  }

  const configurableShurikens = allShurikens.filter(
    s => s.config?.options && Object.keys(s.config.options).length > 0
  )

  if (configurableShurikens.length === 0) {
    return (
      <div className="text-center text-muted-foreground mt-8">
        No Shurikens have configurable fields.
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {configurableShurikens.map(shuriken => (
        <div key={shuriken.metadata.name} className="border rounded-md p-4 space-y-2">
          <div className="flex justify-between items-center">
            <h3 className="text-base font-semibold">{shuriken.metadata.name}</h3>
            <Button
              size="sm"
              onClick={() => handleSave(shuriken.metadata.name)}
              disabled={saving === shuriken.metadata.name}
            >
              <SaveIcon className="mr-2 h-4 w-4" />
              {saving === shuriken.metadata.name ? "Saving..." : "Save"}
            </Button>
          </div>

          {Object.entries(shuriken.config!.options!).map(([key, option]) => (
            <Item variant="outline" className="mb-2" key={key}>
              <ItemContent>
                <ItemTitle>{capitalizeFirstLetter(key)}</ItemTitle>
              </ItemContent>
              <ItemActions>
                {option.type === "Number" ? (
                  <Input type="number" defaultValue={option.value} />
                ) : option.type === "String" ? (
                  <Input type="text" defaultValue={option.value} />
                ) : null}
              </ItemActions>
            </Item>
          ))}
        </div>
      ))}
    </div>
  )
}
