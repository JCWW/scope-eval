// Every configuration through the same pass, side by side.

import CompareArrowsIcon from '@mui/icons-material/CompareArrows';
import {
  Alert,
  Box,
  Button,
  LinearProgress,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { useEffect, useMemo, useRef, useState } from 'react';
import { angle, clock, percent } from '../format';
import { runToEnd, starImage } from '../sim/engine';
import type { Conditions, ConfigSpec, ScenarioSpec, Summary } from '../sim/types';
import { VerdictChip } from './StatTiles';

type Row = { name: string; summary: Summary | null; error: string | null };

interface Props {
  configurations: ConfigSpec[];
  selected: number;
  onSelect: (index: number) => void;
  scenario: ScenarioSpec | null;
  passIndex: number | null;
  conditions: Conditions;
}

export function ComparisonTable({ configurations, selected, onSelect, scenario, passIndex, conditions }: Props) {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const run = useRef(0);

  // Results belong to one scenario, pass and set of configurations.
  const inputs = JSON.stringify([configurations, scenario, passIndex]);
  // Star sizes need no simulation, so they follow the seeing without a re-run.
  const stars = useMemo(
    () =>
      configurations.map((c) => {
        try {
          return starImage(c, conditions);
        } catch {
          return null;
        }
      }),
    [configurations, conditions],
  );
  useEffect(() => {
    run.current++;
    setRows(null);
    setProgress(null);
  }, [inputs]);

  const compare = async () => {
    if (!scenario || passIndex === null) return;
    const id = ++run.current;
    const out: Row[] = [];
    setRows([]);
    for (let i = 0; i < configurations.length; i++) {
      setProgress(i / configurations.length);
      // Yield so the progress bar paints between runs.
      await new Promise((r) => setTimeout(r, 0));
      if (run.current !== id) return;
      const c = configurations[i]!;
      try {
        out.push({ name: c.name, summary: runToEnd(c, scenario, passIndex), error: null });
      } catch (e) {
        out.push({ name: c.name, summary: null, error: e instanceof Error ? e.message : String(e) });
      }
      setRows([...out]);
    }
    setProgress(null);
  };

  return (
    <Paper sx={{ p: 2 }}>
      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1} sx={{ justifyContent: 'space-between', alignItems: { sm: 'center' } }}>
        <Box>
          <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
            Compare configurations
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Runs every configuration through this pass, with the same ephemeris error and seed.
          </Typography>
        </Box>
        <Button
          variant="outlined"
          startIcon={<CompareArrowsIcon />}
          onClick={compare}
          disabled={!scenario || passIndex === null || progress !== null}
        >
          Compare all
        </Button>
      </Stack>
      {progress !== null && <LinearProgress variant="determinate" value={progress * 100} sx={{ mt: 2 }} />}
      {rows && rows.length > 0 && (
        <TableContainer sx={{ mt: 2 }}>
          <Table size="small" aria-label="Configuration comparison">
            <TableHead>
              <TableRow>
                <TableCell>Configuration</TableCell>
                <TableCell>Verdict</TableCell>
                <TableCell align="right">In field</TableCell>
                <TableCell align="right">Exits</TableCell>
                <TableCell align="right">RMS error</TableCell>
                <TableCell align="right">Largest error</TableCell>
                <TableCell align="right">Peak rate used</TableCell>
                <TableCell align="right">Rate-limited</TableCell>
                <TableCell align="right">Star FWHM</TableCell>
                <TableCell align="right">px across</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.map((r, i) => (
                <TableRow
                  key={r.name + i}
                  hover
                  selected={i === selected}
                  onClick={() => onSelect(i)}
                  sx={{ cursor: 'pointer', '& td': { fontVariantNumeric: 'tabular-nums' } }}
                >
                  <TableCell>{r.name}</TableCell>
                  {r.summary ? (
                    <>
                      <TableCell>
                        <VerdictChip verdict={r.summary.verdict} reason={r.summary.verdict_reason} />
                      </TableCell>
                      <TableCell align="right">{percent(r.summary.in_fov_fraction)}</TableCell>
                      <TableCell align="right">{r.summary.fov_exits}</TableCell>
                      <TableCell align="right">{angle(r.summary.rms_err_arcsec)}</TableCell>
                      <TableCell align="right">{angle(r.summary.max_err_arcsec)}</TableCell>
                      <TableCell align="right">{percent(Math.max(...r.summary.peak_rate_utilization), 0)}</TableCell>
                      <TableCell align="right">{r.summary.rate_limited_s > 0 ? clock(r.summary.rate_limited_s) : '-'}</TableCell>
                    </>
                  ) : (
                    <TableCell colSpan={7}>
                      <Alert severity="error" sx={{ py: 0 }}>
                        {r.error}
                      </Alert>
                    </TableCell>
                  )}
                  <TableCell align="right">{stars[i] ? angle(stars[i].tracked_fwhm.center) : '-'}</TableCell>
                  <TableCell align="right">{stars[i] ? stars[i].pixels_across.toFixed(1) : '-'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}
      {rows && rows.length > 0 && progress === null && (
        <Typography variant="caption" color="text.secondary" component="p" sx={{ mt: 1 }}>
          Select a row to switch to that configuration. Star FWHM is the star in a tracked exposure at the sensor centre
          (seeing, diffraction, optics, detector and tracking jitter); px across is the sampled image in native pixels.
        </Typography>
      )}
    </Paper>
  );
}
