import { Construction } from "lucide-react";
import { Card, CardContent, EmptyState, PageHeader } from "../../components/ui";

export function WorkspacePlaceholder({ title }: { title: string; description: string; references: string }) {
  return <div className="page-shell">
    <PageHeader title={title} />
    <Card><CardContent className="pt-5"><EmptyState icon={<Construction size={24} />} title="功能建设中" /></CardContent></Card>
  </div>;
}
