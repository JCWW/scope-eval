// Start, stop, reset and playback speed.

import PauseIcon from '@mui/icons-material/Pause';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import ReplayIcon from '@mui/icons-material/Replay';
import { Box, Button, LinearProgress, Paper, Stack, ToggleButton, ToggleButtonGroup, Typography } from '@mui/material';
import { clock, utc } from '../format';
import { SPEEDS, type Speed } from '../sim/playback';
import type { Sample, SimInfo } from '../sim/types';
import type { Status } from '../sim/useSimulation';

const STATUS_LABEL: Record<Status, string> = {
  loading: 'Loading',
  ready: 'Ready',
  running: 'Running',
  paused: 'Stopped',
  done: 'Pass complete',
  error: 'Error',
};

interface Props {
  status: Status;
  info: SimInfo | null;
  current: Sample | null;
  speed: Speed;
  onSpeed: (s: Speed) => void;
  onStart: () => void;
  onStop: () => void;
  onReset: () => void;
}

export function TransportBar({ status, info, current, speed, onSpeed, onStart, onStop, onReset }: Props) {
  const t = current?.t_s ?? 0;
  const duration = info?.duration_s ?? 0;
  const running = status === 'running';
  const disabled = status === 'loading' || status === 'error';

  return (
    <Paper sx={{ p: 2 }}>
      <Stack direction={{ xs: 'column', md: 'row' }} spacing={2} sx={{ alignItems: { xs: 'stretch', md: 'center' } }}>
        <Stack direction="row" spacing={1}>
          {running ? (
            <Button variant="contained" startIcon={<PauseIcon />} onClick={onStop} sx={{ minWidth: 110 }}>
              Stop
            </Button>
          ) : (
            <Button variant="contained" startIcon={<PlayArrowIcon />} onClick={onStart} disabled={disabled} sx={{ minWidth: 110 }}>
              {status === 'done' ? 'Run again' : status === 'paused' ? 'Resume' : 'Start'}
            </Button>
          )}
          <Button variant="outlined" startIcon={<ReplayIcon />} onClick={onReset} disabled={disabled}>
            Reset
          </Button>
        </Stack>
        <Box sx={{ flexGrow: 1, minWidth: 0 }}>
          <Stack direction="row" sx={{ justifyContent: 'space-between', mb: 0.5 }}>
            <Typography variant="body2" sx={{ fontVariantNumeric: 'tabular-nums' }}>
              {STATUS_LABEL[status]} · {clock(t)} / {clock(duration)}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ fontVariantNumeric: 'tabular-nums' }}>
              {current ? utc(current.utc) : ''}
            </Typography>
          </Stack>
          <LinearProgress
            variant="determinate"
            value={duration > 0 ? Math.min(100, (t / duration) * 100) : 0}
            aria-label="Pass progress"
          />
        </Box>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <Typography variant="body2" color="text.secondary">
            Speed
          </Typography>
          <ToggleButtonGroup
            size="small"
            exclusive
            value={speed}
            onChange={(_, v: Speed | null) => v && onSpeed(v)}
            aria-label="Playback speed"
          >
            {SPEEDS.map((s) => (
              <ToggleButton key={s} value={s} sx={{ px: 1.25 }}>
                {s}x
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </Stack>
      </Stack>
    </Paper>
  );
}
