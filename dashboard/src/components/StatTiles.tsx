// Headline numbers for the run so far.

import CancelIcon from '@mui/icons-material/Cancel';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import WarningIcon from '@mui/icons-material/Warning';
import { Chip, Grid, Paper, Tooltip, Typography } from '@mui/material';
import type { ReactNode } from 'react';
import { angle, clock, percent } from '../format';
import type { Sample, SimInfo, Summary, Verdict } from '../sim/types';

function Tile({ label, value, detail }: { label: string; value: ReactNode; detail?: ReactNode }) {
  return (
    <Paper sx={{ p: 1.5, height: '100%' }}>
      <Typography variant="caption" color="text.secondary" component="div">
        {label}
      </Typography>
      <Typography variant="h6" component="div" sx={{ lineHeight: 1.3 }}>
        {value}
      </Typography>
      {detail && (
        <Typography variant="caption" color="text.secondary" component="div">
          {detail}
        </Typography>
      )}
    </Paper>
  );
}

export function VerdictChip({ verdict, reason }: { verdict: Verdict; reason?: string }) {
  const chip = {
    pass: <Chip size="small" color="success" variant="outlined" icon={<CheckCircleIcon />} label="PASS" />,
    warn: <Chip size="small" color="warning" variant="outlined" icon={<WarningIcon />} label="WARN" />,
    fail: <Chip size="small" color="error" variant="outlined" icon={<CancelIcon />} label="FAIL" />,
  }[verdict];
  return reason ? <Tooltip title={reason}>{chip}</Tooltip> : chip;
}

export function StatTiles({ info, current, summary }: { info: SimInfo; current: Sample | null; summary: Summary | null }) {
  const scale = info.hardware.optics.plate_scale_arcsec;
  const tiles = [
    <Tile
      key="now"
      label="Pointing error now"
      value={current ? angle(current.err_arcsec) : '-'}
      detail={current ? `${(current.err_arcsec / scale).toFixed(0)} px` : undefined}
    />,
    <Tile key="rms" label="RMS error" value={summary ? angle(summary.rms_err_arcsec) : '-'} detail="over the pass so far" />,
    <Tile
      key="max"
      label="Largest error"
      value={summary ? angle(summary.max_err_arcsec) : '-'}
      detail={summary ? `at ${clock(summary.max_err_at_s)}` : undefined}
    />,
    <Tile
      key="fov"
      label="Time in field"
      value={summary ? percent(summary.in_fov_fraction) : '-'}
      detail={summary ? `left the field ${summary.fov_exits} time${summary.fov_exits === 1 ? '' : 's'}` : undefined}
    />,
    <Tile
      key="rate"
      label="Peak axis rate used"
      value={summary ? percent(Math.max(...summary.peak_rate_utilization), 0) : '-'}
      detail={summary && summary.rate_limited_s > 0 ? `rate-limited for ${clock(summary.rate_limited_s)}` : 'never rate-limited'}
    />,
    <Tile
      key="verdict"
      label={summary?.complete ? 'Verdict' : 'Verdict so far'}
      value={summary ? <VerdictChip verdict={summary.verdict} /> : '-'}
      detail={summary?.verdict_reason}
    />,
  ];
  return (
    <Grid container spacing={1.5}>
      {tiles.map((t) => (
        <Grid key={t.key} size={{ xs: 6, sm: 4, lg: 2 }}>
          {t}
        </Grid>
      ))}
    </Grid>
  );
}
