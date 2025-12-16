"use client";

import { useEffect, useState } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";

interface LatencyData {
  time: string;
  p50: number;
  p95: number;
  p99: number;
}

export default function LatencyMetrics() {
  const [data, setData] = useState<LatencyData[]>([]);

  useEffect(() => {
    // Generate mock latency data
    const generateData = () => {
      const now = new Date();
      const newData: LatencyData[] = [];

      for (let i = 9; i >= 0; i--) {
        const time = new Date(now.getTime() - i * 60000);
        newData.push({
          time: time.toLocaleTimeString("en-US", {
            hour: "2-digit",
            minute: "2-digit",
          }),
          p50: Math.random() * 2 + 1,
          p95: Math.random() * 3 + 2,
          p99: Math.random() * 5 + 3,
        });
      }
      return newData;
    };

    setData(generateData());
    const interval = setInterval(() => {
      setData((prev) => {
        const newPoint: LatencyData = {
          time: new Date().toLocaleTimeString("en-US", {
            hour: "2-digit",
            minute: "2-digit",
          }),
          p50: Math.random() * 2 + 1,
          p95: Math.random() * 3 + 2,
          p99: Math.random() * 5 + 3,
        };
        return [...prev.slice(1), newPoint];
      });
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const avgP99 =
    data.length > 0 ? data.reduce((sum, d) => sum + d.p99, 0) / data.length : 0;

  return (
    <div className="bg-slate-800/50 backdrop-blur-sm rounded-lg p-6 border border-slate-700">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-white">Latency Metrics</h2>
        <div className="text-right">
          <div className="text-sm text-slate-400">Avg P99</div>
          <div
            className={`text-2xl font-bold ${
              avgP99 < 5 ? "text-green-400" : "text-yellow-400"
            }`}
          >
            {avgP99.toFixed(2)}ms
          </div>
        </div>
      </div>
      <div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#475569" />
            <XAxis
              dataKey="time"
              stroke="#94a3b8"
              style={{ fontSize: "12px" }}
            />
            <YAxis
              stroke="#94a3b8"
              style={{ fontSize: "12px" }}
              label={{ value: "ms", angle: -90, position: "insideLeft" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1e293b",
                border: "1px solid #475569",
                borderRadius: "8px",
              }}
            />
            <ReferenceLine y={5} stroke="#ef4444" strokeDasharray="3 3" />
            <Line
              type="monotone"
              dataKey="p50"
              stroke="#3b82f6"
              strokeWidth={2}
              dot={false}
              name="P50"
            />
            <Line
              type="monotone"
              dataKey="p95"
              stroke="#f59e0b"
              strokeWidth={2}
              dot={false}
              name="P95"
            />
            <Line
              type="monotone"
              dataKey="p99"
              stroke="#ef4444"
              strokeWidth={2}
              dot={false}
              name="P99"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <div className="flex gap-4 mt-4 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-blue-500" />
          <span className="text-slate-300">P50</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-amber-500" />
          <span className="text-slate-300">P95</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-red-500" />
          <span className="text-slate-300">P99</span>
        </div>
        <div className="flex items-center gap-2 ml-auto">
          <div className="w-3 h-3 border border-red-500 border-dashed" />
          <span className="text-slate-400">Target: &lt;5ms</span>
        </div>
      </div>
    </div>
  );
}
