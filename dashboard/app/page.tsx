"use client";

import { useEffect, useState } from "react";
import PaymentFlows from "@/components/PaymentFlows";
import RiskScoring from "@/components/RiskScoring";
import LatencyMetrics from "@/components/LatencyMetrics";
import SystemStatus from "@/components/SystemStatus";
import Header from "@/components/Header";

export default function Home() {
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    // Check gateway connection
    const checkConnection = async () => {
      try {
        const res = await fetch("/api/swift/health");
        if (res.ok) {
          const data = await res.json();
          setIsConnected(data.status === "healthy");
        } else {
          setIsConnected(false);
        }
      } catch (error) {
        setIsConnected(false);
      }
    };

    checkConnection();
    // Poll every 5 seconds
    const interval = setInterval(checkConnection, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
      <Header isConnected={isConnected} />
      <div className="container mx-auto px-4 py-8">
        <div className="mb-8">
          <h1 className="text-4xl font-bold text-white mb-2">
            ⚡ Swift-RS Dashboard
          </h1>
          <p className="text-slate-300">
            Real-time financial messaging monitoring and analytics
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
          <SystemStatus isConnected={isConnected} />
          <LatencyMetrics />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
          <PaymentFlows />
          <RiskScoring />
        </div>
      </div>
    </main>
  );
}
