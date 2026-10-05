// What a star looks like on the selected configuration's sensor: every term
// of the point spread function, and the star drawn on the pixel grid.
// The numbers come from scope-eval's model through the engine
// (docs/17-point-spread-function.md); only the drawing is done here.

import { Alert, Box, Chip, Paper, Stack, Table, TableBody, TableCell, TableRow, Typography } from '@mui/material';
import { angle, percent } from '../format';
import type { CenterCorner, StarImage } from '../sim/types';
import { useVizColors } from '../theme';
import { LegendKey } from './SkyPlot';

const GRID_PX = 220;

/** Error function, Abramowitz and Stegun 7.1.26, as in calculations/psf.rs. */
export function erf(x: number): number {
  const t = 1 / (1 + 0.3275911 * Math.abs(x));
  const poly = t * (0.254829592 + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
  return Math.sign(x) * (1 - poly * Math.exp(-x * x));
}

/** Share of a 1-D Gaussian (sigma in pixels) falling in the pixel `i` pixels from its centre. */
export function pixelShare(sigmaPx: number, i: number): number {
  const edge = (x: number) => 0.5 * (1 + erf(x / (Math.SQRT2 * sigmaPx)));
  return edge(i + 0.5) - edge(i - 0.5);
}

const pair = (v: CenterCorner) => `${v.center.toFixed(2)}" / ${v.corner.toFixed(2)}"`;
const flat = (v: number) => `${v.toFixed(2)}"`;

function Row({ label, value, strong, note }: { label: string; value: string; strong?: boolean; note?: string }) {
  return (
    <TableRow>
      <TableCell sx={{ pl: 0, fontWeight: strong ? 600 : undefined }}>
        {label}
        {note && (
          <Typography variant="caption" color="text.secondary" component="div">
            {note}
          </Typography>
        )}
      </TableCell>
      <TableCell align="right" sx={{ pr: 0, fontVariantNumeric: 'tabular-nums', fontWeight: strong ? 600 : undefined, whiteSpace: 'nowrap' }}>
        {value}
      </TableCell>
    </TableRow>
  );
}

/** The star centred on a pixel, shaded by the share of light each pixel collects. */
function PixelGrid({ star }: { star: StarImage }) {
  const c = useVizColors();
  const scale = star.plate_scale_arcsec;
  const sigmaPx = star.sampled_fwhm.center / scale / 2.35482;
  // Wide enough for the tracked star, odd so a pixel sits at the centre.
  const want = Math.ceil((2 * star.tracked_fwhm.center) / scale);
  const n = Math.min(21, Math.max(5, want % 2 === 0 ? want + 1 : want));
  const half = (n - 1) / 2;
  const cell = GRID_PX / n;
  const shares = Array.from({ length: n }, (_, k) => pixelShare(sigmaPx, k - half));
  const peak = shares[half]! * shares[half]!;
  const r = (fwhm: number) => (fwhm / scale / 2) * cell;
  const mid = GRID_PX / 2;

  const cells = [];
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      const f = shares[i]! * shares[j]!;
      cells.push(
        <rect
          key={`${i}-${j}`}
          x={j * cell}
          y={i * cell}
          width={cell}
          height={cell}
          fill={c.series1}
          fillOpacity={Math.min(1, f / peak)}
          stroke={c.grid}
          strokeWidth={1}
        />,
      );
    }
  }
  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'center' }}>
        <svg
          viewBox={`0 0 ${GRID_PX} ${GRID_PX}`}
          width="100%"
          style={{ maxWidth: GRID_PX }}
          role="img"
          aria-label={`A star centred on a pixel: ${star.pixels_across.toFixed(1)} pixels across its FWHM, ${percent(star.peak_pixel_fraction.center)} of its light in the brightest pixel`}
        >
          {cells}
          <circle cx={mid} cy={mid} r={r(star.recorded_fwhm.center)} fill="none" stroke={c.ink} strokeWidth={1.5} />
          {star.jitter_arcsec > 0 && (
            <circle cx={mid} cy={mid} r={r(star.tracked_fwhm.center)} fill="none" stroke={c.series2} strokeWidth={2} strokeDasharray="4 3" />
          )}
        </svg>
      </Box>
      <Stack direction="row" spacing={2} sx={{ justifyContent: 'center', flexWrap: 'wrap', mt: 1 }} useFlexGap>
        <LegendKey color={c.ink} ring label="Recorded FWHM" />
        {star.jitter_arcsec > 0 && <LegendKey color={c.series2} dashed label="With tracking jitter" />}
      </Stack>
      <Typography variant="caption" color="text.secondary" component="p" align="center">
        {n} x {n} native pixels of {scale.toFixed(2)}"; shading is the light each pixel collects
      </Typography>
    </Box>
  );
}

export function StarImagePanel({ star, error }: { star: StarImage | null; error: string | null }) {
  return (
    <Paper sx={{ p: 2 }}>
      <Typography variant="subtitle1" component="h2" sx={{ fontWeight: 600 }}>
        Star image
      </Typography>
      <Typography variant="body2" color="text.secondary" gutterBottom>
        FWHM at the sensor centre / corner{star ? ` (${star.corner_radius_mm.toFixed(1)} mm off-axis)` : ''}
      </Typography>
      {error && <Alert severity="error">{error}</Alert>}
      {star && (
        <Stack spacing={1.5}>
          <Table size="small" aria-label="Point spread function budget">
            <TableBody>
              <Row label="Seeing" value={flat(star.seeing_arcsec)} />
              <Row label="Diffraction" value={flat(star.diffraction_arcsec)} />
              {star.optics_arcsec ? (
                <Row
                  label="Optics"
                  value={pair(star.optics_arcsec)}
                  note={`spot read as ${star.spot_reading}${star.optics_extrapolated ? ', corner extrapolated' : ''}`}
                />
              ) : (
                <Row label="Optics" value="no spot data" />
              )}
              <Row label="Detector diffusion" value={star.diffusion_arcsec === null ? 'no MTF' : flat(star.diffusion_arcsec)} />
              <Row label="Sampled image" value={pair(star.sampled_fwhm)} strong note={`${star.pixels_across.toFixed(2)} px across at the centre`} />
              <Row label="Pixel aperture" value={flat(star.pixel_arcsec)} />
              <Row label="Recorded star" value={pair(star.recorded_fwhm)} strong />
              <Row
                label="Tracking jitter"
                value={flat(star.jitter_arcsec)}
                note={star.jitter_assumed ? 'default 1" RMS, assumed' : 'from the mount figures'}
              />
              <Row label="In a tracked exposure" value={pair(star.tracked_fwhm)} strong />
            </TableBody>
          </Table>
          <PixelGrid star={star} />
          <Typography variant="body2" color="text.secondary">
            Brightest pixel holds {percent(star.peak_pixel_fraction.center)} of the light with the star centred on it,{' '}
            {percent(star.peak_pixel_fraction.corner)} on a pixel corner. Largest term: {star.largest_term}.
          </Typography>
          {star.sampled_fwhm_if_diameter && (
            <Typography variant="body2" color="text.secondary">
              The spot size doesn't say whether it is an RMS radius or diameter. As a diameter the sampled image would be{' '}
              {angle(star.sampled_fwhm_if_diameter.center)} ({(star.sampled_fwhm_if_diameter.center / star.plate_scale_arcsec).toFixed(2)} px).
            </Typography>
          )}
          {star.assumed.length > 0 && (
            <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap' }} useFlexGap>
              <Typography variant="caption" color="text.secondary" sx={{ mr: 0.5 }}>
                Assumed or missing:
              </Typography>
              {star.assumed.map((a) => (
                <Chip key={a} size="small" variant="outlined" label={a} />
              ))}
            </Stack>
          )}
        </Stack>
      )}
    </Paper>
  );
}
