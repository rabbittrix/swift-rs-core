import { useEffect } from "react";
import { CompliancePanel } from "@/components/CompliancePanel";
import { Shell } from "@/components/Shell";
import { NetworkDashboard } from "@/components/NetworkDashboard";
import { PaymentSimulator } from "@/components/PaymentSimulator";
import { WalletView } from "@/components/WalletView";
import { subscribeLive } from "@/lib/api";
import { useDesk } from "@/store/useDesk";

export default function App() {
  const view = useDesk((state) => state.view);
  const hydrate = useDesk((state) => state.hydrate);
  const pushLive = useDesk((state) => state.pushLive);

  useEffect(() => {
    void hydrate();
    let stop = () => {};
    let alive = true;
    void subscribeLive((tx) => {
      if (alive) pushLive(tx);
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => {
      alive = false;
      stop();
    };
  }, [hydrate, pushLive]);

  return (
    <Shell>
      {view === "simulator" && <PaymentSimulator />}
      {view === "network" && <NetworkDashboard />}
      {view === "wallet" && <WalletView />}
      {view === "compliance" && <CompliancePanel />}
    </Shell>
  );
}
