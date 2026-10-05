// Material UI theme, with light and dark schemes that follow the operating
// system until the viewer picks one.
//
// Chart colors come from a validated reference palette: two series slots
// (blue, orange) that pass color-vision-deficiency separation and 3:1
// contrast in both modes, and status colors reserved for meaning
// (in field / out of field), always paired with an icon and a label.

import { createTheme } from '@mui/material/styles';
import { useColorScheme } from '@mui/material/styles';

const LIGHT = {
  page: '#f9f9f7',
  surface: '#fcfcfb',
  ink: '#0b0b0b',
  inkSecondary: '#52514e',
  muted: '#898781',
  grid: '#e1e0d9',
  axis: '#c3c2b7',
  series1: '#2a78d6',
  series2: '#eb6834',
};

const DARK: typeof LIGHT = {
  page: '#0d0d0d',
  surface: '#1a1a19',
  ink: '#ffffff',
  inkSecondary: '#c3c2b7',
  muted: '#898781',
  grid: '#2c2c2a',
  axis: '#383835',
  series1: '#3987e5',
  series2: '#d95926',
};

export const STATUS = {
  good: '#0ca30c',
  warning: '#fab219',
  critical: '#d03b3b',
};

export type VizColors = typeof LIGHT;

export const theme = createTheme({
  cssVariables: { colorSchemeSelector: 'class' },
  colorSchemes: {
    light: {
      palette: {
        primary: { main: LIGHT.series1 },
        background: { default: LIGHT.page, paper: LIGHT.surface },
        text: { primary: LIGHT.ink, secondary: LIGHT.inkSecondary },
        divider: LIGHT.grid,
        success: { main: STATUS.good },
        warning: { main: STATUS.warning },
        error: { main: STATUS.critical },
      },
    },
    dark: {
      palette: {
        primary: { main: DARK.series1 },
        background: { default: DARK.page, paper: DARK.surface },
        text: { primary: DARK.ink, secondary: DARK.inkSecondary },
        divider: DARK.grid,
        success: { main: STATUS.good },
        warning: { main: STATUS.warning },
        error: { main: STATUS.critical },
      },
    },
  },
  typography: {
    fontFamily: 'system-ui, -apple-system, "Segoe UI", sans-serif',
  },
  shape: { borderRadius: 8 },
  components: {
    MuiPaper: { defaultProps: { variant: 'outlined' } },
    MuiButton: { defaultProps: { disableElevation: true } },
  },
});

/** The chart palette for the scheme actually on screen. */
export function useVizColors(): VizColors {
  const { mode, systemMode } = useColorScheme();
  const resolved = mode === 'system' ? systemMode : mode;
  return resolved === 'dark' ? DARK : LIGHT;
}
