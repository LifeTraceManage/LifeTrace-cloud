/*
 * LifeTrace finance is intentionally a thin mount point.
 *
 * The actual finance presentation is the source-derived BeeCount Cloud Web port
 * under ./beecount-cloud. Do not rebuild finance screens with LifeTrace generic
 * UI components here: LifeTrace owns the shared WorkspaceShell/session while
 * BeeCount owns the finance information architecture and presentation.
 */
import { WalletCards } from "lucide-react";
import { WorkspaceShell } from "../../layouts/WorkspaceShell";
import { BeeCountCloudWorkspace } from "./beecount-cloud/BeeCountCloudWorkspace";

export function FinanceWorkspace() {
  return (
    <WorkspaceShell
      title="Finance"
      description="账本、交易、账户、预算与统计"
      icon={<WalletCards size={17} />}
    >
      <div className="page-shell">
        <h1 className="sr-only">财务</h1>
        <span className="sr-only">唯一财务数据源</span>
        <BeeCountCloudWorkspace />
      </div>
    </WorkspaceShell>
  );
}
