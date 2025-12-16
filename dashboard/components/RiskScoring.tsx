"use client";

import { useEffect, useState } from "react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";

interface RiskScore {
  level: string;
  count: number;
  percentage: number;
}

export default function RiskScoring() {
  const [riskData, setRiskData] = useState<RiskScore[]>([]);
  const [totalAnalyzed, setTotalAnalyzed] = useState(0);

  useEffect(() => {
    const generateRiskData = () => {
      const low = Math.floor(Math.random() * 50) + 30;
      const medium = Math.floor(Math.random() * 30) + 10;
      const high = Math.floor(Math.random() * 15) + 5;
      const critical = Math.floor(Math.random() * 5) + 1;

      const total = low + medium + high + critical;
      setTotalAnalyzed(total);

      return [
        { level: "Low", count: low, percentage: (low / total) * 100 },
        { level: "Medium", count: medium, percentage: (medium / total) * 100 },
        { level: "High", count: high, percentage: (high / total) * 100 },
        {
          level: "Critical",
          count: critical,
          percentage: (critical / total) * 100,
        },
      ];
    };

    setRiskData(generateRiskData());
    const interval = setInterval(() => {
      setRiskData(generateRiskData());
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const getColor = (level: string) => {
    switch (level) {
      case "Low":
        return "#10b981";
      case "Medium":
        return "#f59e0b";
      case "High":
        return "#f97316";
      case "Critical":
        return "#ef4444";
      default:
        return "#64748b";
    }
  };

  return (
    <div className="bg-slate-800/50 backdrop-blur-sm rounded-lg p-6 border border-slate-700">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-white">AI Risk Scoring</h2>
        <div className="text-right">
          <div className="text-sm text-slate-400">Total Analyzed</div>
          <div className="text-2xl font-bold text-white">{totalAnalyzed}</div>
        </div>
      </div>
      <div className="h-64 mb-4">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={riskData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#475569" />
            <XAxis
              dataKey="level"
              stroke="#94a3b8"
              style={{ fontSize: "12px" }}
            />
            <YAxis
              stroke="#94a3b8"
              style={{ fontSize: "12px" }}
              label={{ value: "Count", angle: -90, position: "insideLeft" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1e293b",
                border: "1px solid #475569",
                borderRadius: "8px",
              }}
              formatter={(value: number, name: string) => [
                `${value} (${riskData
                  .find((d) => d.level === name)
                  ?.percentage.toFixed(1)}%)`,
                "Count",
              ]}
            />
            <Bar dataKey="count" radius={[8, 8, 0, 0]}>
              {riskData.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={getColor(entry.level)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
      <div className="grid grid-cols-4 gap-2">
        {riskData.map((item) => (
          <div
            key={item.level}
            className="bg-slate-700/50 rounded-lg p-3 text-center"
          >
            <div
              className="text-sm font-medium mb-1"
              style={{ color: getColor(item.level) }}
            >
              {item.level}
            </div>
            <div className="text-xl font-bold text-white">{item.count}</div>
            <div className="text-xs text-slate-400">
              {item.percentage.toFixed(1)}%
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
