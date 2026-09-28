import { createTheme } from "@mui/material/styles";

export const theme = createTheme({
  palette: {
    mode: "dark",
    primary: {
      main: "#9146ff",
    },
    secondary: {
      main: "#6a5cff",
    },
    background: {
      default: "#0f1117",
      paper: "#171a24",
    },
    success: {
      main: "#91f5b8",
    },
    warning: {
      main: "#ffcc66",
    },
    error: {
      main: "#ff7b64",
    },
    text: {
      primary: "#eef2ff",
      secondary: "#b8c0d9",
    },
    divider: "#2b3142",
  },
  typography: {
    fontFamily: '"IBM Plex Sans", "Segoe UI", sans-serif',
    h1: { fontWeight: 700 },
    h2: { fontWeight: 600 },
    h3: { fontWeight: 600 },
    button: { textTransform: "none", fontWeight: 600 },
  },
  shape: {
    borderRadius: 16,
  },
  components: {
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: "none",
          border: "1px solid #2b3142",
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          fontFamily: '"IBM Plex Mono", monospace',
        },
      },
    },
  },
});
