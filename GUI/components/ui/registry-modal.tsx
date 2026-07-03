"use client";

import { useConfig } from "@/hooks/config";
import { useState } from "react";
import { Button } from "./button";
import { Input } from "./input";
import { Trash, Plus, Save, Pencil } from "lucide-react";

import { Item, ItemContent, ItemTitle } from "./item";

function RegistryRow({
  name,
  url,
  removeRegistry,
}: {
  name: string;
  url: string;
  removeRegistry: (name: string) => Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(url);

  return (
    <Item className="flex items-center justify-between gap-3">
      {/* LEFT SIDE */}
      <ItemContent className="flex flex-col gap-1">
        <ItemTitle>{name}</ItemTitle>

        {editing ? (
          <Input
            value={value}
            onChange={(e) => setValue(e.target.value)}
            className="h-8"
          />
        ) : (
          <span className="text-xs text-muted-foreground break-all">{url}</span>
        )}
      </ItemContent>

      {/* ACTIONS */}
      <div className="flex gap-2">
        {editing ? (
          <Button
            size="icon"
            onClick={() => {
              // TODO: add updateRegistry to store
              setEditing(false);
            }}
          >
            <Save />
          </Button>
        ) : (
          <Button
            size="icon"
            variant="outline"
            onClick={() => setEditing(true)}
          >
            <Pencil />
          </Button>
        )}

        <Button
          size="icon"
          variant="destructive"
          onClick={() => removeRegistry(name)}
        >
          <Trash />
        </Button>
      </div>
    </Item>
  );
}

export default function RegistryModal() {
  const { config, addRegistry, removeRegistry } = useConfig();

  const [newName, setNewName] = useState("");
  const [newUrl, setNewUrl] = useState("");

  const registries = config?.registries ? Array.from(config.registries) : [];

  return (
    <div className="w-full max-w-2xl space-y-4">
      <h2 className="text-2xl font-semibold">Registries</h2>

      {/* ADD */}
      <div className="flex gap-2 p-3 border rounded-lg bg-background">
        <Input
          placeholder="name (npm, cargo...)"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <Input
          placeholder="url"
          value={newUrl}
          onChange={(e) => setNewUrl(e.target.value)}
        />
        <Button
          onClick={async () => {
            if (!newName || !newUrl) return;

            await addRegistry(newName, newUrl);
            setNewName("");
            setNewUrl("");
          }}
        >
          <Plus />
        </Button>
      </div>

      {/* LIST */}
      <div className="space-y-2">
        {registries.length === 0 && (
          <div className="text-sm text-muted-foreground p-3 border rounded-lg">
            no registries yet
          </div>
        )}

        {registries.map(([name, url]) => (
          <RegistryRow
            key={name}
            name={name}
            url={url}
            removeRegistry={removeRegistry}
          />
        ))}
      </div>
    </div>
  );
}
