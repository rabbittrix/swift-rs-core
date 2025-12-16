"use client";

import { useEffect, useState } from "react";

interface HeaderProps {
  isConnected: boolean;
}

export default function Header({ isConnected }: HeaderProps) {
  const [currentTime, setCurrentTime] = useState(new Date());

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  return (
    <header className="bg-slate-800/50 backdrop-blur-sm border-b border-slate-700">
      <div className="container mx-auto px-4 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <div
                className={`w-3 h-3 rounded-full ${
                  isConnected ? "bg-green-500 animate-pulse" : "bg-red-500"
                }`}
              />
              <span className="text-sm text-slate-300">
                {isConnected ? "Connected" : "Disconnected"}
              </span>
            </div>
            <span className="text-sm text-slate-400">
              {currentTime.toLocaleTimeString()}
            </span>
          </div>
          <div className="flex items-center gap-4">
            <span className="text-sm text-slate-300">Swift-RS v0.1.0</span>
          </div>
        </div>
      </div>
    </header>
  );
}
