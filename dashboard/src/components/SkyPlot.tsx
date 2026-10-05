// The sky as seen from the site: zenith in the center, horizon at the rim,
// north up and east to the left (as when lying on your back looking up).
// Shows the predicted pass, the path flown so far, where the satellite
// really is and where the telescope really points.

import { Box, Paper, Stack, Typography } from '@mui/material';
import type { Sample, TrackPoint } from '../sim/types';
import { useVizColors } from '../theme';

const SIZE = 320;
const C = SIZE / 2;
const R = 136;

/** Polar position: radius grows from the zenith (0) to the horizon (R). */
function xy(azDeg: number, elDeg: number): [number, number] {
  const r = (Math.max(0, 90 - elDeg) / 90) * R;
  const a = (azDeg * Math.PI) / 180;
  // East to the left: looking up, north at the top puts east on the left.
  return [C - r * Math.sin(a), C - r * Math.cos(a)];
}

function path(points: [number, number][]): string {
  return points.map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`).join('');
}

export function SkyPlot({ track, samples, current }: { track: TrackPoint[]; samples: Sample[]; current: Sample | null }) {
  const c = useVizColors();
  const predicted = path(track.map((p) => xy(p.az_deg, p.el_deg)));
  const flown = path(samples.map((s) => xy(s.target_az_deg, s.target_el_deg)));
  const target = current ? xy(current.target_az_deg, current.target_el_deg) : null;
  const scope = current ? xy(current.boresight_az_deg, current.boresight_el_deg) : null;

  return (
    <Paper sx={{ p: 2, height: '100%' }}>
      <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
        Sky view
      </Typography>
      <Typography variant="body2" color="text.secondary">
        Zenith at the center, horizon at the rim
      </Typography>
      <Box sx={{ display: 'flex', justifyContent: 'center', mt: 1 }}>
        <svg
          viewBox={`0 0 ${SIZE} ${SIZE}`}
          width="100%"
          style={{ maxWidth: 360 }}
          role="img"
          aria-label="Sky plot of the satellite pass, the satellite's position and the telescope's pointing"
        >
          {[0, 30, 60].map((el) => (
            <g key={el}>
              <circle cx={C} cy={C} r={((90 - el) / 90) * R} fill="none" stroke={el === 0 ? c.axis : c.grid} strokeWidth={1} />
              <text x={C + 3} y={C - ((90 - el) / 90) * R + 12} fontSize={10} fill={c.muted}>
                {el} deg
              </text>
            </g>
          ))}
          <line x1={C - R} y1={C} x2={C + R} y2={C} stroke={c.grid} strokeWidth={1} />
          <line x1={C} y1={C - R} x2={C} y2={C + R} stroke={c.grid} strokeWidth={1} />
          {(
            [
              ['N', 0],
              ['E', 90],
              ['S', 180],
              ['W', 270],
            ] as const
          ).map(([label, az]) => {
            const [x, y] = xy(az, -9);
            return (
              <text key={label} x={x} y={y + 4} fontSize={12} textAnchor="middle" fill={c.inkSecondary}>
                {label}
              </text>
            );
          })}
          <path d={predicted} fill="none" stroke={c.muted} strokeWidth={2} strokeDasharray="4 4" />
          <path d={flown} fill="none" stroke={c.series1} strokeWidth={2} />
          {scope && (
            <circle cx={scope[0]} cy={scope[1]} r={7} fill="none" stroke={c.series2} strokeWidth={2}>
              <title>Telescope pointing</title>
            </circle>
          )}
          {target && (
            <circle cx={target[0]} cy={target[1]} r={4.5} fill={c.series1} stroke={c.surface} strokeWidth={2}>
              <title>Satellite</title>
            </circle>
          )}
        </svg>
      </Box>
      <Stack direction="row" spacing={2} sx={{ justifyContent: 'center', flexWrap: 'wrap', mt: 1 }}>
        <LegendKey color={c.muted} dashed label="Predicted pass" />
        <LegendKey color={c.series1} label="Satellite" />
        <LegendKey color={c.series2} ring label="Telescope pointing" />
      </Stack>
      {current && (
        <Typography variant="body2" color="text.secondary" align="center" sx={{ mt: 1, fontVariantNumeric: 'tabular-nums' }}>
          Satellite az {current.target_az_deg.toFixed(1)} deg, el {current.target_el_deg.toFixed(1)} deg
        </Typography>
      )}
    </Paper>
  );
}

export function LegendKey({ color, label, dashed, ring }: { color: string; label: string; dashed?: boolean; ring?: boolean }) {
  return (
    <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center' }}>
      <svg width={18} height={10} aria-hidden>
        {ring ? (
          <circle cx={9} cy={5} r={4} fill="none" stroke={color} strokeWidth={2} />
        ) : (
          <line x1={1} y1={5} x2={17} y2={5} stroke={color} strokeWidth={2} strokeDasharray={dashed ? '4 3' : undefined} />
        )}
      </svg>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
    </Stack>
  );
}
