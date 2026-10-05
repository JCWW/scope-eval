// Time series: pointing error, and how hard each mount axis is working.
// One y-axis per chart, a crosshair tooltip on hover, and the limit that
// matters drawn as a reference line.

import { Paper, Typography } from '@mui/material';
import { ChartsReferenceLine } from '@mui/x-charts/ChartsReferenceLine';
import { LineChart } from '@mui/x-charts/LineChart';
import { angle, clock } from '../format';
import { decimate } from '../sim/playback';
import type { Sample, SimInfo } from '../sim/types';
import { useVizColors } from '../theme';

const MAX_POINTS = 1200;

/** Log-axis ticks in plain arcseconds: 1, 10, 100, 1k, 10k, 1M. */
export function arcsecTick(v: number): string {
  if (v >= 1e6) return `${v / 1e6}M`;
  if (v >= 1e3) return `${v / 1e3}k`;
  return String(v);
}
const HEIGHT = 260;

function xAxis(info: SimInfo, data: number[]) {
  return [
    {
      data,
      scaleType: 'linear' as const,
      min: 0,
      max: info.duration_s,
      label: 'Time since rise',
      valueFormatter: (v: number) => clock(v),
    },
  ];
}

export function ErrorChart({ info, samples }: { info: SimInfo; samples: Sample[] }) {
  const c = useVizColors();
  const points = decimate(samples, MAX_POINTS, (s) => s.err_arcsec);
  const edge = (Math.min(info.hardware.optics.fov_w_deg, info.hardware.optics.fov_h_deg) * 3600) / 2;
  const peak = points.reduce((m, s) => Math.max(m, s.err_arcsec), 0);
  const low = points.reduce((m, s) => Math.min(m, s.err_arcsec), Infinity);
  // Log scale: errors run from a few arcseconds to degrees near a keyhole.
  const min = 10 ** Math.floor(Math.log10(Math.max(0.1, Math.min(low, edge) || 1)));
  const max = Math.min(1e6, 10 ** Math.ceil(Math.log10(Math.max(peak, edge) * 1.2)));

  return (
    <Paper sx={{ p: 2 }}>
      <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
        Pointing error
      </Typography>
      <Typography variant="body2" color="text.secondary">
        Angle between the satellite and the boresight. Above the dashed line the satellite is off the sensor's short side.
      </Typography>
      <LineChart
        height={HEIGHT}
        skipAnimation
        hideLegend
        xAxis={xAxis(info, points.map((s) => s.t_s))}
        yAxis={[{ scaleType: 'log', min, max, label: 'arcsec', width: 56, valueFormatter: arcsecTick }]}
        series={[
          {
            id: 'error',
            label: 'Pointing error',
            data: points.map((s) => Math.max(s.err_arcsec, min)),
            color: c.series1,
            showMark: false,
            valueFormatter: (v) => (v === null ? '-' : angle(v)),
          },
        ]}
        grid={{ horizontal: true }}
        margin={{ left: 8, right: 16 }}
      >
        <ChartsReferenceLine
          y={edge}
          label={`Field edge (${angle(edge)})`}
          labelAlign="start"
          lineStyle={{ stroke: c.muted, strokeDasharray: '6 4', strokeWidth: 1.5 }}
          labelStyle={{ fill: c.inkSecondary, fontSize: 12 }}
        />
      </LineChart>
    </Paper>
  );
}

export function RateChart({ info, samples }: { info: SimInfo; samples: Sample[] }) {
  const c = useVizColors();
  const points = decimate(samples, MAX_POINTS, (s) => Math.max(Math.abs(s.axis1_rate_deg_s), Math.abs(s.axis2_rate_deg_s)));
  const limit = info.hardware.mount_model.max_rate_deg_s;
  const peak = points.reduce((m, s) => Math.max(m, Math.abs(s.axis1_rate_deg_s), Math.abs(s.axis2_rate_deg_s)), 0);
  // Show the limit when it is within reach of the data; a 50 deg/s limit
  // over a 1 deg/s pass would flatten the lines into the axis.
  const showLimit = limit.value <= Math.max(peak, 0.05) * 4;
  const max = showLimit ? limit.value * 1.08 : Math.max(peak * 1.15, 0.05);
  const [a1, a2] = info.axis_names;

  return (
    <Paper sx={{ p: 2 }}>
      <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
        Axis rates
      </Typography>
      <Typography variant="body2" color="text.secondary">
        {showLimit
          ? `How fast each axis turns, against the mount's ${limit.value} deg/s limit${limit.assumed ? ' (assumed)' : ''}.`
          : `How fast each axis turns. The ${limit.value} deg/s limit${limit.assumed ? ' (assumed)' : ''} is far above this pass's needs.`}
      </Typography>
      <LineChart
        height={HEIGHT}
        skipAnimation
        xAxis={xAxis(info, points.map((s) => s.t_s))}
        yAxis={[{ min: 0, max, label: 'deg/s', width: 56, valueFormatter: (v: number) => v.toFixed(2) }]}
        series={[
          {
            id: 'axis1',
            label: a1,
            data: points.map((s) => Math.abs(s.axis1_rate_deg_s)),
            color: c.series1,
            showMark: false,
            valueFormatter: (v) => (v === null ? '-' : `${v.toFixed(3)} deg/s`),
          },
          {
            id: 'axis2',
            label: a2,
            data: points.map((s) => Math.abs(s.axis2_rate_deg_s)),
            color: c.series2,
            showMark: false,
            valueFormatter: (v) => (v === null ? '-' : `${v.toFixed(3)} deg/s`),
          },
        ]}
        grid={{ horizontal: true }}
        margin={{ left: 8, right: 16 }}
      >
        {showLimit && (
          <ChartsReferenceLine
            y={limit.value}
            label="Axis rate limit"
            labelAlign="start"
            lineStyle={{ stroke: c.muted, strokeDasharray: '6 4', strokeWidth: 1.5 }}
            labelStyle={{ fill: c.inkSecondary, fontSize: 12 }}
          />
        )}
      </LineChart>
    </Paper>
  );
}
