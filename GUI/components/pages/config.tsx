"use client"

import { useEffect, useState } from "react"
import { Input } from "../ui/input"
import { Button } from "../ui/button"
import { SaveIcon } from "lucide-react"
import { Item, ItemActions, ItemContent, ItemTitle } from "../ui/item"
import { useShuriken } from "@/hooks/use-shuriken"
import { invoke } from "@tauri-apps/api/core"
import { Switch } from "../ui/switch"

function capitalizeFirstLetter(str: string) {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

// Render input based on plain JS type
function renderInput(
  shurikenName: string,
  key: string,
  value: any,
  optionsState: Record<string, Record<string, any>>,
  setOptionsState: React.Dispatch<React.SetStateAction<Record<string, Record<string, any>>>>
) {
  const current = optionsState[shurikenName]?.[key]

  const handleChange = (v: any) => {
    setOptionsState(prev => ({
      ...prev,
      [shurikenName]: {
        ...prev[shurikenName],
        [key]: v
      }
    }))
  }

  const type = typeof current

  if (type === "string") {
    return <Input type="text" value={current} onChange={e => handleChange(e.target.value)} />
  } else if (type === "number") {
    return <Input type="number" value={current} onChange={e => handleChange(Number(e.target.value))} />
  } else if (type === "boolean") {
    return <Switch checked={current} onCheckedChange={handleChange} />
  } else if (Array.isArray(current) || type === "object") {
    return <pre className="text-xs">{JSON.stringify(current, null, 2)}</pre>
  } else {
    return null
  }
}

export default function Configuration() {
  const { allShurikens, refreshShurikens } = useShuriken()
  const [saving, setSaving] = useState<string | null>(null)

  const [optionsState, setOptionsState] = useState<Record<string, Record<string, any>>>({})

  useEffect(() => {
    refreshShurikens()
  }, [refreshShurikens])

  // Initialize local state
  useEffect(() => {
    const newState: Record<string, Record<string, any>> = {}
    allShurikens.forEach(s => {
      if (s.config?.options) {
        newState[s.metadata.name] = { ...s.config.options } // plain JS values
      }
    })
    setOptionsState(newState)
  }, [allShurikens])

  const handleSave = async (shurikenName: string) => {
    try {
      setSaving(shurikenName)
      await invoke("save_config", { name: shurikenName, data: optionsState[shurikenName] })
      await invoke("configure_shuriken", { name: shurikenName })
      await refreshShurikens()
      console.log(optionsState[shurikenName])
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

          {Object.entries(shuriken.config!.options!).map(([key, value]) => (
            <Item variant="outline" className="mb-2" key={key}>
              <ItemContent>
                <ItemTitle>{capitalizeFirstLetter(key)}</ItemTitle>
              </ItemContent>
              <ItemActions>
                {renderInput(
                  shuriken.metadata.name,
                  key,
                  value,
                  optionsState,
                  setOptionsState
                )}
              </ItemActions>
            </Item>
          ))}
        </div>
      ))}
    </div>
  )
}
