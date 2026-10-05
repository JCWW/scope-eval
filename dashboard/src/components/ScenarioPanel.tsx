// What to observe, from where, and which pass.

import { Alert, Box, FormControl, InputLabel, MenuItem, Paper, Select, Stack, TextField, Typography } from '@mui/material';
import { CUSTOM_TLE_ID, TARGETS } from '../config/scenarios';
import { duration, utc } from '../format';
import type { PassList, SiteSpec } from '../sim/types';
import { NumberField } from './NumberField';

export interface ScenarioSettings {
  min_el_deg: number;
  ephemeris_error_km: number;
  seed: number;
}

interface Props {
  targetId: string;
  onTarget: (id: string) => void;
  customTle: string;
  onCustomTle: (text: string) => void;
  customStart: string;
  onCustomStart: (iso: string) => void;
  site: SiteSpec;
  onSite: (site: SiteSpec) => void;
  settings: ScenarioSettings;
  onSettings: (s: ScenarioSettings) => void;
  passes: PassList | null;
  passError: string | null;
  passIndex: number | null;
  onPass: (index: number) => void;
}

export function ScenarioPanel(p: Props) {
  const preset = TARGETS.find((t) => t.id === p.targetId);
  return (
    <Paper sx={{ p: 2 }}>
      <Typography variant="subtitle1" component="h2" gutterBottom sx={{ fontWeight: 600 }}>
        Target and pass
      </Typography>
      <Stack spacing={1.5}>
        <FormControl size="small" fullWidth>
          <InputLabel id="target-select">Target</InputLabel>
          <Select labelId="target-select" label="Target" value={p.targetId} onChange={(e) => p.onTarget(e.target.value)}>
            {TARGETS.map((t) => (
              <MenuItem key={t.id} value={t.id}>
                {t.label}
              </MenuItem>
            ))}
            <MenuItem value={CUSTOM_TLE_ID}>Paste a TLE…</MenuItem>
          </Select>
        </FormControl>
        {preset && (
          <Typography variant="body2" color="text.secondary">
            {preset.description}
          </Typography>
        )}
        {p.targetId === CUSTOM_TLE_ID && (
          <>
            <TextField
              label="TLE (two or three lines)"
              multiline
              minRows={3}
              value={p.customTle}
              onChange={(e) => p.onCustomTle(e.target.value)}
              slotProps={{ htmlInput: { style: { fontFamily: 'ui-monospace, monospace', fontSize: 12 }, spellCheck: false } }}
              helperText="From CelesTrak or Space-Track. The search starts at the time below."
            />
            <TextField
              size="small"
              label="Search start, UTC"
              value={p.customStart}
              onChange={(e) => p.onCustomStart(e.target.value)}
              helperText="YYYY-MM-DDTHH:MM, within two weeks of the TLE's epoch"
            />
          </>
        )}
        <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 1.5 }}>
          <NumberField label="Latitude" value={p.site.lat_deg} min={-90} max={90} onCommit={(v) => p.onSite({ ...p.site, lat_deg: v ?? 0 })} />
          <NumberField label="Longitude" value={p.site.lon_deg} min={-180} max={360} onCommit={(v) => p.onSite({ ...p.site, lon_deg: v ?? 0 })} />
          <NumberField label="Altitude, m" value={p.site.alt_m} min={-500} max={9000} onCommit={(v) => p.onSite({ ...p.site, alt_m: v ?? 0 })} />
        </Box>
        <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 1.5 }}>
          <NumberField
            label="Min elev, deg"
            value={p.settings.min_el_deg}
            min={0}
            max={80}
            onCommit={(v) => p.onSettings({ ...p.settings, min_el_deg: v ?? 10 })}
          />
          <NumberField
            label="Ephem. error, km"
            value={p.settings.ephemeris_error_km}
            min={-1000}
            max={1000}
            onCommit={(v) => p.onSettings({ ...p.settings, ephemeris_error_km: v ?? 0 })}
            helperText="along-track"
          />
          <NumberField
            label="Seed"
            value={p.settings.seed}
            min={0}
            onCommit={(v) => p.onSettings({ ...p.settings, seed: Math.round(v ?? 1) })}
            helperText="pointing draw"
          />
        </Box>

        {p.passError && <Alert severity="error">{p.passError}</Alert>}
        {p.passes?.warnings.map((w) => (
          <Alert key={w} severity="warning">
            {w}
          </Alert>
        ))}
        {p.passes && p.passes.passes.length === 0 && !p.passError && (
          <Alert severity="info">No passes above the minimum elevation in this window.</Alert>
        )}
        {p.passes && p.passes.passes.length > 0 && p.passIndex !== null && (
          <FormControl size="small" fullWidth>
            <InputLabel id="pass-select">Pass</InputLabel>
            <Select labelId="pass-select" label="Pass" value={p.passIndex} onChange={(e) => p.onPass(Number(e.target.value))}>
              {p.passes.passes.map((pass) => (
                <MenuItem key={pass.index} value={pass.index}>
                  <Box>
                    <Typography variant="body2">
                      {utc(pass.rise)} · max {pass.max_el_deg.toFixed(0)} deg
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {duration(pass.duration_s)} · satellite {pass.lighting} · site {pass.site_dark}
                    </Typography>
                  </Box>
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        )}
      </Stack>
    </Paper>
  );
}
