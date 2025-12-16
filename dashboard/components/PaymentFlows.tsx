"use client";

import { useEffect, useState } from "react";
import { format } from "date-fns";

interface PaymentFlow {
  id: string;
  sender: string;
  receiver: string;
  amount: string;
  currency: string;
  status: "pending" | "processing" | "completed" | "failed";
  timestamp: Date;
  messageType: "MT" | "MX";
}

export default function PaymentFlows() {
  const [flows, setFlows] = useState<PaymentFlow[]>([]);

  useEffect(() => {
    // Generate mock payment flows
    const generateMockFlow = (): PaymentFlow => {
      const statuses: PaymentFlow["status"][] = [
        "pending",
        "processing",
        "completed",
        "failed",
      ];
      const types: PaymentFlow["messageType"][] = ["MT", "MX"];
      const currencies = ["USD", "EUR", "GBP", "JPY"];

      return {
        id: `MSG-${Math.random().toString(36).substr(2, 9).toUpperCase()}`,
        sender: `BANK${Math.floor(Math.random() * 1000)}`,
        receiver: `BANK${Math.floor(Math.random() * 1000)}`,
        amount: (Math.random() * 1000000).toFixed(2),
        currency: currencies[Math.floor(Math.random() * currencies.length)],
        status: statuses[Math.floor(Math.random() * statuses.length)],
        timestamp: new Date(),
        messageType: types[Math.floor(Math.random() * types.length)],
      };
    };

    // Initial data
    const initialFlows: PaymentFlow[] = Array.from({ length: 10 }, () =>
      generateMockFlow()
    );
    setFlows(initialFlows);

    // Add new flow every 3 seconds
    const interval = setInterval(() => {
      setFlows((prev) => [generateMockFlow(), ...prev.slice(0, 19)]);
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  const getStatusColor = (status: PaymentFlow["status"]) => {
    switch (status) {
      case "completed":
        return "bg-green-500/20 text-green-400";
      case "processing":
        return "bg-blue-500/20 text-blue-400";
      case "pending":
        return "bg-yellow-500/20 text-yellow-400";
      case "failed":
        return "bg-red-500/20 text-red-400";
    }
  };

  return (
    <div className="bg-slate-800/50 backdrop-blur-sm rounded-lg p-6 border border-slate-700">
      <h2 className="text-xl font-semibold text-white mb-4">
        Real-time Payment Flows
      </h2>
      <div className="space-y-2 max-h-96 overflow-y-auto">
        {flows.map((flow) => (
          <div
            key={flow.id}
            className="bg-slate-700/50 rounded-lg p-4 border border-slate-600/50 hover:border-slate-500 transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-white font-mono text-sm">{flow.id}</span>
                <span
                  className={`px-2 py-0.5 rounded text-xs font-medium ${getStatusColor(
                    flow.status
                  )}`}
                >
                  {flow.status}
                </span>
                <span className="px-2 py-0.5 rounded text-xs font-medium bg-slate-600 text-slate-200">
                  {flow.messageType}
                </span>
              </div>
              <span className="text-slate-400 text-xs">
                {format(flow.timestamp, "HH:mm:ss")}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <div className="flex items-center gap-2">
                <span className="text-slate-300">{flow.sender}</span>
                <span className="text-slate-500">→</span>
                <span className="text-slate-300">{flow.receiver}</span>
              </div>
              <div className="text-white font-semibold">
                {flow.amount} {flow.currency}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
