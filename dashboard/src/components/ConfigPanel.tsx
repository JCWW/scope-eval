// Switch between configurations, and edit the selected one.

import AddIcon from '@mui/icons-material/Add';
import DeleteIcon from '@mui/icons-material/DeleteOutlined';
import RestartAltIcon from '@mui/icons-material/RestartAlt';
import {
  Alert,
  Box,
  Button,
  Chip,
  FormControl,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  TextField,
  Typography,
} from '@mui/material';
import type { ConfigSpec, Hardware, MountOverrides, Param, Presets } from '../sim/types';
import { NumberField } from './NumberField';

interface Props {
  configurations: ConfigSpec[];
  selected: number;
  onSelect: (index: number) => void;
  onChange: (config: ConfigSpec) => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onRestore: () => void;
  presets: Presets;
  hardware: Hardware | null;
  error: string | null;
}

function PresetSelect({ label, value, options, onChange }: { label: string; value: string; options: string[]; onChange: (v: string) => void }) {
  const id = `preset-${label.toLowerCase()}`;
  return (
    <FormControl size="small" fullWidth>
      <InputLabel id={id}>{label}</InputLabel>
      <Select labelId={id} label={label} value={options.includes(value) ? value : ''} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <MenuItem key={o} value={o}>
            {o}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}

function source(p: Param | [Param, Param] | undefined, override: number | null | undefined): string {
  if (override != null) return 'entered here';
  if (!p) return '';
  if (Array.isArray(p)) {
    const [a, b] = p;
    if (a.value !== b.value || a.source !== b.source) return `per axis: ${a.value} / ${b.value}`;
    p = a;
  }
  if (p.source === 'measured') return `blank: measured ${p.value}`;
  return p.assumed ? `blank: ${p.value} assumed` : `blank: preset value ${p.value}`;
}

export function ConfigPanel(props: Props) {
  const { configurations, selected, onSelect, onChange, presets, hardware, error } = props;
  const config = configurations[selected];
  if (!config) return null;
  const o = config.mount_overrides;
  const m = hardware?.mount_model;
  const setOverride = (key: keyof MountOverrides, value: number | null) =>
    onChange({ ...config, mount_overrides: { ...o, [key]: value } });
  const assumed = m
    ? [
        m.max_rate_deg_s.some((p) => p.assumed) && 'axis rate',
        m.max_accel_deg_s2.some((p) => p.assumed) && 'axis acceleration',
        m.pointing_rms_arcsec.assumed && 'pointing RMS',
        m.jitter_rms_arcsec.assumed && 'jitter',
        m.kind_assumed && 'mount type',
      ].filter(Boolean)
    : [];

  return (
    <Paper sx={{ p: 2 }}>
      <Typography variant="subtitle1" component="h2" gutterBottom sx={{ fontWeight: 600 }}>
        Configuration
      </Typography>
      <FormControl fullWidth size="small">
        <InputLabel id="config-select">Active configuration</InputLabel>
        <Select labelId="config-select" label="Active configuration" value={selected} onChange={(e) => onSelect(Number(e.target.value))}>
          {configurations.map((c, i) => (
            <MenuItem key={i} value={i}>
              {c.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
      <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', mt: 1 }} useFlexGap>
        <Button size="small" startIcon={<AddIcon />} onClick={props.onDuplicate}>
          Duplicate
        </Button>
        <Button size="small" startIcon={<DeleteIcon />} onClick={props.onDelete} disabled={configurations.length <= 1}>
          Delete
        </Button>
        <Button size="small" startIcon={<RestartAltIcon />} onClick={props.onRestore}>
          Restore defaults
        </Button>
      </Stack>

      <Stack spacing={1.5} sx={{ mt: 2 }}>
        <TextField
          size="small"
          label="Name"
          value={config.name}
          onChange={(e) => onChange({ ...config, name: e.target.value })}
        />
        <PresetSelect
          label="Telescope"
          value={config.telescope}
          options={presets.telescopes.map((t) => t.name)}
          onChange={(v) => onChange({ ...config, telescope: v })}
        />
        <PresetSelect
          label="Camera"
          value={config.camera}
          options={presets.cameras.map((c) => c.name)}
          onChange={(v) => onChange({ ...config, camera: v })}
        />
        <PresetSelect
          label="Mount"
          value={config.mount}
          options={presets.mounts.map((m) => m.name)}
          onChange={(v) => onChange({ ...config, mount: v })}
        />
        <Typography variant="body2" color="text.secondary">
          Mount figures. Leave blank to use the preset, or a stated default where the preset has none.
        </Typography>
        <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1.5 }}>
          <NumberField
            label="Max rate, deg/s"
            value={o.max_rate_deg_s}
            allowEmpty
            min={0.001}
            onCommit={(v) => setOverride('max_rate_deg_s', v)}
            helperText={source(m?.max_rate_deg_s, o.max_rate_deg_s)}
          />
          <NumberField
            label="Max accel, deg/s²"
            value={o.max_accel_deg_s2}
            allowEmpty
            min={0.001}
            onCommit={(v) => setOverride('max_accel_deg_s2', v)}
            helperText={source(m?.max_accel_deg_s2, o.max_accel_deg_s2)}
          />
          <NumberField
            label='Pointing RMS, "'
            value={o.pointing_rms_arcsec}
            allowEmpty
            min={0}
            onCommit={(v) => setOverride('pointing_rms_arcsec', v)}
            helperText={source(m?.pointing_rms_arcsec, o.pointing_rms_arcsec)}
          />
          <NumberField
            label='Jitter RMS, "'
            value={o.jitter_rms_arcsec}
            allowEmpty
            min={0}
            onCommit={(v) => setOverride('jitter_rms_arcsec', v)}
            helperText={source(m?.jitter_rms_arcsec, o.jitter_rms_arcsec)}
          />
        </Box>
        {error && <Alert severity="error">{error}</Alert>}
        {hardware && (
          <Box>
            <Typography variant="body2" color="text.secondary">
              {hardware.mount_model.kind === 'alt_az' ? 'Alt-az' : 'Equatorial'} mount ·{' '}
              {hardware.optics.fov_w_deg.toFixed(2)} x {hardware.optics.fov_h_deg.toFixed(2)} deg field ·{' '}
              {hardware.optics.plate_scale_arcsec.toFixed(2)}"/px
            </Typography>
            {assumed.length > 0 && (
              <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap', mt: 1 }} useFlexGap>
                <Typography variant="caption" color="text.secondary" sx={{ mr: 0.5 }}>
                  Assumed:
                </Typography>
                {assumed.map((a) => (
                  <Chip key={String(a)} size="small" variant="outlined" label={a} />
                ))}
              </Stack>
            )}
          </Box>
        )}
      </Stack>
    </Paper>
  );
}
