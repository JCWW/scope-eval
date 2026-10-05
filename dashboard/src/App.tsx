import DarkModeIcon from '@mui/icons-material/DarkMode';
import LightModeIcon from '@mui/icons-material/LightMode';
import SettingsBrightnessIcon from '@mui/icons-material/SettingsBrightness';
import {
  Alert,
  AppBar,
  Box,
  CircularProgress,
  Container,
  Grid,
  IconButton,
  Stack,
  Toolbar,
  Tooltip,
  Typography,
} from '@mui/material';
import { useColorScheme } from '@mui/material/styles';
import { useEffect, useMemo, useState } from 'react';
import { ErrorChart, RateChart } from './components/Charts';
import { ComparisonTable } from './components/ComparisonTable';
import { ConfigPanel } from './components/ConfigPanel';
import { FieldView } from './components/FieldView';
import { ScenarioPanel, type ScenarioSettings } from './components/ScenarioPanel';
import { SkyPlot } from './components/SkyPlot';
import { StatTiles } from './components/StatTiles';
import { TransportBar } from './components/TransportBar';
import { DEFAULT_CONFIGURATIONS } from './config/configurations';
import { CUSTOM_TLE_ID, DEFAULT_SCENARIO_SETTINGS, DEFAULT_SITE, TARGETS, scenarioFor } from './config/scenarios';
import { duration } from './format';
import { findPasses, getPresets, loadEngine, resolveConfig } from './sim/engine';
import type { ConfigSpec, PassList, Presets, ScenarioSpec, SiteSpec } from './sim/types';
import { useSimulation } from './sim/useSimulation';

const STORAGE_KEY = 'scope-sim-dashboard:configurations';

// Edited configurations are a per-browser convenience: if storage is
// unavailable the dashboard simply starts from the defaults.
function loadConfigurations(): ConfigSpec[] {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as unknown;
    if (Array.isArray(saved) && saved.length > 0) return saved as ConfigSpec[];
  } catch {
    // fall through to the defaults
  }
  return DEFAULT_CONFIGURATIONS;
}

function saveConfigurations(configs: ConfigSpec[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(configs));
  } catch {
    // not saved; nothing else to do
  }
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

function highestPass(list: PassList | null): number | null {
  if (!list || list.passes.length === 0) return null;
  return list.passes.reduce((a, b) => (b.max_el_deg > a.max_el_deg ? b : a)).index;
}

function ModeToggle() {
  const { mode, setMode } = useColorScheme();
  const next = mode === 'system' ? 'light' : mode === 'light' ? 'dark' : 'system';
  const icon = mode === 'light' ? <LightModeIcon /> : mode === 'dark' ? <DarkModeIcon /> : <SettingsBrightnessIcon />;
  return (
    <Tooltip title={`Theme: ${mode ?? 'system'} (switch to ${next})`}>
      <IconButton color="inherit" onClick={() => setMode(next)} aria-label={`Switch to ${next} theme`}>
        {icon}
      </IconButton>
    </Tooltip>
  );
}

export function App() {
  const [engineError, setEngineError] = useState<string | null>(null);
  const [presets, setPresets] = useState<Presets | null>(null);
  useEffect(() => {
    loadEngine()
      .then(() => setPresets(getPresets()))
      .catch((e: unknown) => setEngineError(message(e)));
  }, []);
  const ready = presets !== null;

  // Configurations.
  const [configs, setConfigs] = useState<ConfigSpec[]>(loadConfigurations);
  const [selected, setSelected] = useState(0);
  useEffect(() => saveConfigurations(configs), [configs]);
  const config = configs[Math.min(selected, configs.length - 1)] ?? null;
  const resolved = useMemo(() => {
    if (!ready || !config) return { hardware: null, error: null };
    try {
      return { hardware: resolveConfig(config), error: null };
    } catch (e) {
      return { hardware: null, error: message(e) };
    }
  }, [ready, config]);

  // Scenario.
  const [targetId, setTargetId] = useState(TARGETS[0]!.id);
  const [customTle, setCustomTle] = useState('');
  const [customStart, setCustomStart] = useState(() => new Date().toISOString().slice(0, 16));
  const [site, setSite] = useState<SiteSpec>(DEFAULT_SITE);
  const [settings, setSettings] = useState<ScenarioSettings>(DEFAULT_SCENARIO_SETTINGS);
  const scenario: ScenarioSpec = useMemo(() => {
    const preset = TARGETS.find((t) => t.id === targetId);
    if (preset) return scenarioFor(preset, site, settings);
    return { site, target: { kind: 'tle', text: customTle }, start: customStart, hours: 24, ...settings };
  }, [targetId, customTle, customStart, site, settings]);
  const scenarioKey = JSON.stringify(scenario);

  const passes = useMemo(() => {
    if (!ready) return { list: null, error: null };
    if (targetId === CUSTOM_TLE_ID && customTle.trim() === '') return { list: null, error: null };
    try {
      return { list: findPasses(scenario), error: null };
    } catch (e) {
      return { list: null, error: message(e) };
    }
    // scenarioKey stands in for scenario, so equal contents don't search again.
  }, [ready, scenarioKey]);

  // The chosen pass belongs to one scenario; a new scenario starts on its highest pass.
  const [passChoice, setPassChoice] = useState<{ key: string; index: number } | null>(null);
  const passIndex = passChoice?.key === scenarioKey ? passChoice.index : highestPass(passes.list);

  const sim = useSimulation(
    ready,
    resolved.error ? null : config,
    passes.list && passes.list.passes.length > 0 ? scenario : null,
    passIndex,
  );
  const { view } = sim;

  const updateConfig = (c: ConfigSpec) => setConfigs((all) => all.map((x, i) => (i === selected ? c : x)));
  const duplicate = () => {
    if (!config) return;
    setConfigs((all) => [...all, { ...config, name: `${config.name} (copy)` }]);
    setSelected(configs.length);
  };
  const remove = () => {
    if (configs.length <= 1) return;
    setConfigs((all) => all.filter((_, i) => i !== selected));
    setSelected((s) => Math.max(0, s - 1));
  };
  const restore = () => {
    setConfigs(DEFAULT_CONFIGURATIONS);
    setSelected(0);
  };

  return (
    <Box sx={{ minHeight: '100vh', bgcolor: 'background.default' }}>
      <AppBar position="static" color="transparent" elevation={0} sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Toolbar>
          <Box sx={{ flexGrow: 1, minWidth: 0 }}>
            <Typography variant="h6" component="h1" noWrap>
              Scope Sim
            </Typography>
            <Typography variant="caption" color="text.secondary" noWrap component="p">
              A telescope mount tracking a satellite pass, and where the satellite lands on the sensor
            </Typography>
          </Box>
          <ModeToggle />
        </Toolbar>
      </AppBar>

      <Container maxWidth="xl" sx={{ py: 2, px: { xs: 2, sm: 3 } }}>
        {engineError && (
          <Alert severity="error" sx={{ mb: 2 }}>
            The simulation engine failed to load: {engineError}. Run <code>npm run wasm</code> and reload.
          </Alert>
        )}
        {!ready && !engineError && (
          <Stack direction="row" spacing={2} sx={{ alignItems: 'center', py: 6, justifyContent: 'center' }}>
            <CircularProgress size={24} />
            <Typography>Loading the simulation engine…</Typography>
          </Stack>
        )}
        {ready && presets && (
          <Grid container spacing={2}>
            <Grid size={{ xs: 12, md: 4, lg: 3 }}>
              <Stack spacing={2}>
                <ConfigPanel
                  configurations={configs}
                  selected={Math.min(selected, configs.length - 1)}
                  onSelect={setSelected}
                  onChange={updateConfig}
                  onDuplicate={duplicate}
                  onDelete={remove}
                  onRestore={restore}
                  presets={presets}
                  hardware={resolved.hardware}
                  error={resolved.error}
                />
                <ScenarioPanel
                  targetId={targetId}
                  onTarget={setTargetId}
                  customTle={customTle}
                  onCustomTle={setCustomTle}
                  customStart={customStart}
                  onCustomStart={setCustomStart}
                  site={site}
                  onSite={setSite}
                  settings={settings}
                  onSettings={setSettings}
                  passes={passes.list}
                  passError={passes.error}
                  passIndex={passIndex}
                  onPass={(index) => setPassChoice({ key: scenarioKey, index })}
                />
              </Stack>
            </Grid>

            <Grid size={{ xs: 12, md: 8, lg: 9 }}>
              <Stack spacing={2}>
                <TransportBar
                  status={sim.status}
                  info={view.info}
                  current={view.current}
                  speed={sim.speed}
                  onSpeed={sim.setSpeed}
                  onStart={sim.start}
                  onStop={sim.stop}
                  onReset={sim.reset}
                />
                {view.error && <Alert severity="error">{view.error}</Alert>}
                {view.info?.truncated && (
                  <Alert severity="info">
                    This pass lasts {duration(view.info.pass.duration_s)}; the first {duration(view.info.duration_s)} are
                    simulated.
                  </Alert>
                )}
                {view.info && (
                  <>
                    <StatTiles info={view.info} current={view.current} summary={view.summary} />
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, lg: 5 }}>
                        <SkyPlot track={view.track} samples={view.samples} current={view.current} />
                      </Grid>
                      <Grid size={{ xs: 12, lg: 7 }}>
                        <FieldView info={view.info} samples={view.samples} current={view.current} />
                      </Grid>
                    </Grid>
                    <ErrorChart info={view.info} samples={view.samples} />
                    <RateChart info={view.info} samples={view.samples} />
                  </>
                )}
                <ComparisonTable
                  configurations={configs}
                  selected={selected}
                  onSelect={setSelected}
                  scenario={passes.list && passes.list.passes.length > 0 ? scenario : null}
                  passIndex={passIndex}
                />
              </Stack>
            </Grid>
          </Grid>
        )}
      </Container>
    </Box>
  );
}
