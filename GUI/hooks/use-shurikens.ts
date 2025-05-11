// src/hooks/useShurikens.ts
import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect } from 'react';

interface Shuriken {
  id: string;
  name: string;
  binary_path: string;
  config_path: string;
  status: 'running' | 'stopped';
  pid?: number;
}

export function useShurikens() {
  const [shurikens, setShurikens] = useState<Shuriken[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    setLoading(true);
    try {
      const result = await invoke<Shuriken[]>('get_all_shurikens');
      setShurikens(result);
    } finally {
      setLoading(false);
    }
  };

  const addShuriken = async (shuriken: Omit<Shuriken, 'status' | 'pid'>) => {
    await invoke('create_shuriken', {
      id: shuriken.id,
      name: shuriken.name,
      binary_path: shuriken.binary_path,
      config_path: shuriken.config_path,
    });
    await refresh();
  };

  const toggleStatus = async (id: string, currentStatus: 'running' | 'stopped') => {
    const newStatus = currentStatus === 'running' ? 'stopped' : 'running';
    await invoke('update_shuriken_status', {
      id,
      status: newStatus,
      pid: newStatus === 'running' ? Math.floor(Math.random() * 10000) : null,
    });
    await refresh();
  };

  useEffect(() => {
    refresh();
  }, []);

  return { shurikens, loading, refresh, addShuriken, toggleStatus };
}