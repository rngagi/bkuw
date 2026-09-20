import { Type } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";

export type FontStatus = "checking" | "ready" | "attention";

interface Props {
  status: FontStatus;
  onClick(): void;
}

export function FontManagerButton({ status, onClick }: Props) {
  const { t } = useTranslation();
  const label = t(`fontSetup.status.${status}`);

  return (
    <Button
      className="font-manager-button"
      data-state={status}
      size="icon"
      type="button"
      variant="ghost"
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      <Type size={17} />
      {status === "attention" && <span className="font-attention-dot" aria-hidden="true" />}
    </Button>
  );
}
