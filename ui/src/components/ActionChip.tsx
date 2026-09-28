import { Chip } from "@mui/material";

type Props = {
  action: string;
};

export default function ActionChip({ action }: Props) {
  const normalized = action.toLowerCase();
  const color =
    normalized === "pass"
      ? "success"
      : normalized === "flag"
        ? "warning"
        : normalized === "block"
          ? "error"
          : "default";

  return <Chip size="small" label={action} color={color} variant="outlined" />;
}
