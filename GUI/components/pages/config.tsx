"use client"

import { Dispatch, SetStateAction, useState } from "react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Switch } from "@/components/ui/switch"
import { Input } from "../ui/input"
import { Button } from "../ui/button"
import { SaveIcon } from "lucide-react"
import { motion } from "motion/react"
import { Shuriken } from "@/lib/types"
import { Item, ItemActions, ItemContent, ItemTitle } from "../ui/item"

const tabs = ["Ninja"]

function capitalizeFirstLetter(str: string) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}


export default function Configuration({configtmp, setConfigtmp, shurikens}: {configtmp?: NinjaConfig, setConfigtmp?: Dispatch<SetStateAction<NinjaConfig>>, shurikens: Shuriken[]}) {
  const [config, setConfig] = useState<NinjaConfig>({
    devMode: false,
    serverurl: "https://ninja-rs.vercel.app",
    checkUpdates: true,
  })

  return (
    <div className="space-y-2">
      <Tabs defaultValue="ninja" className="w-full">
        <div className="flex justify-between">

        <TabsList className={`grid grid-cols-${tabs.length + shurikens.length} max-w-md mb-2 px-2`}>
            <TabsTrigger value={"ninja"} key={"ninja"} className="text-sm font-medium">
              Ninja
            </TabsTrigger>

            {shurikens.map((value, index) => (
              value.config != null?(
              <TabsTrigger value={value.metadata.name} key={value.metadata.name} className="text-sm font-medium">
                  {value.metadata.name}
              </TabsTrigger>): null
            ))}
        </TabsList>
            <Button>
              <SaveIcon className="mr-2 h-4 w-4" />
              Save Configuration
            </Button>
        </div>
        <TabsContent value="ninja">
          
          <Item>
              <ItemContent>
                <span className="text-sm font-medium">Custom server</span>
              </ItemContent>

              <ItemActions>
                  <Input onChange={(e) => setConfig({
                    serverurl: e.target.value,
                    checkUpdates: config.checkUpdates,
                    devMode: config.devMode
                  })} value={config.serverurl} type="url" />
              </ItemActions>
            </Item>

            <Item>
              <ItemContent>
                  <span className="text-sm font-medium">Check for updates</span>
              </ItemContent>      
                <ItemActions>
                  <Switch onCheckedChange={(e) => {
                    setConfig({
                      serverurl: config.serverurl,
                      checkUpdates: e,
                      devMode: config.devMode
                    })
                  }} checked={config.checkUpdates} />
                </ItemActions>
            </Item>
        </TabsContent> 
        {shurikens.map((value, index) => (
          value.config?.options && Object.keys(value.config.options).length > 0 ? (
            <TabsContent value={value.metadata.name} key={index}>
              {Object.entries(value.config.options).map(([key, option], i) => (
                <Item variant={"outline"} className="mb-2">
                  <ItemContent>
                    <ItemTitle>{capitalizeFirstLetter(key)}</ItemTitle>
                  </ItemContent>
                  <ItemActions>
                    {
                      typeof option === "number"?
                      <Input className="w-fit" type="number" defaultValue={option}/>:
                      typeof option === "string"?
                      <Input type="text" defaultValue={option}/>:
                      null
                    }
                  </ItemActions>
                </Item>
              ))}
            </TabsContent>
          ) : (
            <p key={index}>No options to configure.</p>
          )
        ))}

      </Tabs>
    </div>
  )
}
