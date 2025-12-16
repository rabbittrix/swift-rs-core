"use client";

import { useEffect, useState } from "react";
import { format } from "date-fns";

interface SystemStatusProps {
  isConnected: boolean;
}

interface HealthData {
  status: string;
  version: string;
}

export default function SystemStatus({ isConnected }: SystemStatusProps) {
  const [health, setHealth] = useState<HealthData | null>(null);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [connectionStatus, setConnectionStatus] = useState(false);

  useEffect(() => {
    const fetchHealth = async () => {
      try {
        const res = await fetch("/api/swift/health");
        if (res.ok) {
          const data = await res.json();
          setHealth(data);
          setConnectionStatus(data.status === "healthy");
          setLastUpdate(new Date());
        } else {
          setConnectionStatus(false);
        }
      } catch (error) {
        console.error("Failed to fetch health:", error);
        setConnectionStatus(false);
      }
    };

    fetchHealth();
    const interval = setInterval(fetchHealth, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="bg-slate-800/50 backdrop-blur-sm rounded-lg p-6 border border-slate-700">
      <h2 className="text-xl font-semibold text-white mb-4">System Status</h2>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <span className="text-slate-300">Gateway Status</span>
          <span
            className={`px-3 py-1 rounded-full text-sm font-medium ${
              connectionStatus || isConnected
                ? "bg-green-500/20 text-green-400"
                : "bg-red-500/20 text-red-400"
            }`}
          >
            {connectionStatus || isConnected ? "Online" : "Offline"}
          </span>
        </div>
        {health && (
          <>
            <div className="flex items-center justify-between">
              <span className="text-slate-300">Version</span>
              <span className="text-white font-mono">{health.version}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-slate-300">Last Update</span>
              <span className="text-slate-400 text-sm">
                {format(lastUpdate, "HH:mm:ss")}
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
