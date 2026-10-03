import { Star } from "lucide-react";
import { cn } from "../../components/ui";
import type { MailAddress, MailMessageSummary } from "./types";

function formatAddress(address: MailAddress): string {
  return address.name ? `${address.name} <${address.email}>` : address.email;
}

function addressesText(addresses?: MailAddress[]): string {
  return addresses?.map(formatAddress).join(", ") ?? "";
}

export function MailMessageRow({
  message,
  selected,
  onSelect,
}: {
  message: MailMessageSummary;
  selected: boolean;
  onSelect(): void;
}) {
  const unread = !message.isRead;
  return <button
    type="button"
    data-testid={`mail-message-${message.id}`}
    data-read-state={unread ? "unread" : "read"}
    aria-label={`${unread ? "未读" : "已读"}邮件：${message.subject || "(无主题)"}`}
    onClick={onSelect}
    className={cn(
      "mb-1 w-full rounded-md px-3 py-3 text-left transition-colors",
      selected ? "bg-accent" : unread ? "bg-primary/[0.045] hover:bg-primary/[0.08]" : "hover:bg-muted",
    )}
  >
    <div className="flex items-center gap-2">
      <span
        data-testid="mail-unread-indicator"
        className={cn(
          "h-2 w-2 shrink-0 rounded-full",
          unread ? "bg-primary" : "bg-transparent",
        )}
        aria-hidden="true"
      />
      <span className={cn("min-w-0 flex-1 truncate text-xs", unread && "font-semibold text-foreground")}>
        {addressesText(message.from) || "未知发件人"}
      </span>
      {message.isStarred ? <Star size={12} className="shrink-0 fill-current text-warning" /> : null}
      <span className="shrink-0 text-[10px] font-normal text-muted-foreground">
        {new Date(message.sentAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}
      </span>
    </div>
    <div className={cn("mt-1 truncate text-sm", unread && "font-semibold")}>
      {message.subject || "(无主题)"}
    </div>
    <div className="mt-1 line-clamp-2 text-xs font-normal leading-5 text-muted-foreground">
      {message.preview || "无预览"}
    </div>
  </button>;
}
