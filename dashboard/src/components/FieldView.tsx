// The camera's view: the sensor outline, the boresight at its center, and
// where the satellite lands on it. This is the pointing error made visible.

import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import { Box, Chip, Paper, Stack, ToggleButton, ToggleButtonGroup, Typography } from '@mui/material';
import { useState, type ReactNode } from 'react';
import { angle } from '../format';
import { trail } from '../sim/playback';
import type { Sample, SimInfo } from '../sim/types';
import { useVizColors } from '../theme';
import { LegendKey } from './SkyPlot';

const W = 360;
const TRAIL_S = 30;

type Scale = 'field' | 'zoom';

export function FieldView({ info, samples, current }: { info: SimInfo; samples: Sample[]; current: Sample | null }) {
  const c = useVizColors();
  const [scale, setScale] = useState<Scale>('field');
  const { optics, mount_model: m } = info.hardware;

  const halfW = (optics.fov_w_deg * 3600) / 2;
  const halfH = (optics.fov_h_deg * 3600) / 2;
  const aspect = halfH / halfW;
  const H = W * aspect;
  const rms = m.pointing_rms_arcsec.value;
  // Zoomed: about five times the pointing RMS across, never wider than the field.
  const zoomHalf = Math.min(halfW * 1.08, Math.max(60, 5 * (rms + m.jitter_rms_arcsec.value)));
  const viewHalfW = scale === 'field' ? halfW * 1.08 : zoomHalf;
  const viewHalfH = viewHalfW * aspect;

  const px = (x: number) => W / 2 + (x / viewHalfW) * (W / 2);
  const py = (y: number) => H / 2 - (y / viewHalfH) * (H / 2);

  const recent = current ? trail(samples, current.t_s, TRAIL_S) : [];
  const trailPath = recent
    .map((s, i) => `${i === 0 ? 'M' : 'L'}${px(s.err_x_arcsec).toFixed(1)},${py(s.err_y_arcsec).toFixed(1)}`)
    .join('');

  let marker: ReactNode = null;
  if (current) {
    const x = px(current.err_x_arcsec);
    const y = py(current.err_y_arcsec);
    const inside = x >= 0 && x <= W && y >= 0 && y <= H;
    if (inside) {
      marker = <circle cx={x} cy={y} r={5} fill={c.series1} stroke={c.surface} strokeWidth={2} />;
    } else {
      // Off the view: an arrow at the edge, pointing at the satellite.
      const dx = x - W / 2;
      const dy = y - H / 2;
      const k = Math.min((W / 2 - 12) / Math.abs(dx || 1e-9), (H / 2 - 12) / Math.abs(dy || 1e-9));
      const ex = W / 2 + dx * k;
      const ey = H / 2 + dy * k;
      const rot = (Math.atan2(dy, dx) * 180) / Math.PI;
      marker = (
        <g>
          <polygon points="-7,-6 7,0 -7,6" transform={`translate(${ex},${ey}) rotate(${rot})`} fill={c.series1} />
          <text
            x={ex - Math.sign(dx) * 14}
            y={ey - Math.sign(dy) * 14 + 4}
            fontSize={11}
            textAnchor={dx > 0 ? 'end' : 'start'}
            fill={c.ink}
          >
            {angle(current.err_arcsec)} off
          </text>
        </g>
      );
    }
  }

  const inField = current?.in_fov ?? true;

  return (
    <Paper sx={{ p: 2, height: '100%' }}>
      <Stack direction="row" spacing={1} sx={{ justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <Box>
          <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
            Camera field
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {optics.fov_w_deg.toFixed(2)} x {optics.fov_h_deg.toFixed(2)} deg, {optics.plate_scale_arcsec.toFixed(2)}"/px
          </Typography>
        </Box>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <Chip
            size="small"
            icon={inField ? <CheckCircleIcon /> : <ErrorIcon />}
            label={inField ? 'In field' : 'Out of field'}
            color={inField ? 'success' : 'error'}
            variant="outlined"
          />
          <ToggleButtonGroup
            size="small"
            exclusive
            value={scale}
            onChange={(_, v: Scale | null) => v && setScale(v)}
            aria-label="Field view scale"
          >
            <ToggleButton value="field">Whole field</ToggleButton>
            <ToggleButton value="zoom">Zoom</ToggleButton>
          </ToggleButtonGroup>
        </Stack>
      </Stack>
      <Box sx={{ display: 'flex', justifyContent: 'center', mt: 1.5 }}>
        <svg
          viewBox={`0 0 ${W} ${H}`}
          width="100%"
          style={{ maxWidth: 480, overflow: 'hidden' }}
          role="img"
          aria-label="The satellite's position on the camera sensor relative to the telescope's boresight"
        >
          <rect
            x={px(-halfW)}
            y={py(halfH)}
            width={px(halfW) - px(-halfW)}
            height={py(-halfH) - py(halfH)}
            fill="none"
            stroke={c.axis}
            strokeWidth={2}
          />
          <line x1={W / 2 - 8} y1={H / 2} x2={W / 2 + 8} y2={H / 2} stroke={c.muted} strokeWidth={1} />
          <line x1={W / 2} y1={H / 2 - 8} x2={W / 2} y2={H / 2 + 8} stroke={c.muted} strokeWidth={1} />
          {rms > 0 && (
            <circle cx={W / 2} cy={H / 2} r={(rms / viewHalfW) * (W / 2)} fill="none" stroke={c.series2} strokeWidth={2} strokeDasharray="4 4" />
          )}
          <path d={trailPath} fill="none" stroke={c.series1} strokeWidth={2} strokeOpacity={0.45} />
          {marker}
        </svg>
      </Box>
      <Stack direction="row" spacing={2} sx={{ justifyContent: 'center', flexWrap: 'wrap', mt: 1 }}>
        <LegendKey color={c.series1} label={`Satellite, last ${TRAIL_S} s`} />
        <LegendKey color={c.series2} dashed label={`Pointing RMS (${angle(rms)})`} />
        <LegendKey color={c.axis} label="Sensor edge" />
      </Stack>
      {current && (
        <Typography variant="body2" color="text.secondary" align="center" sx={{ mt: 1, fontVariantNumeric: 'tabular-nums' }}>
          {info.axis_names[0]} {angle(current.err_x_arcsec)}, {info.axis_names[1].toLowerCase()} {angle(current.err_y_arcsec)}{' '}
          ({(current.err_arcsec / optics.plate_scale_arcsec).toFixed(0)} px from center)
        </Typography>
      )}
    </Paper>
  );
}
