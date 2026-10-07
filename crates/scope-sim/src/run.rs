//! Repeatable runs: a run file names one configuration, one scenario and
//! one pass; a trace is everything the run recorded.
//!
//! The command-line runner (`src/bin/scope-sim.rs`) and the golden-trace
//! tests both read run files, so any run you can describe you can repeat
//! and pin down. Example run files are in `crates/scope-sim/runs/`.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::presets::{ConfigSpec, Presets};
use crate::scenario::{find_passes, PassList, ScenarioSpec};
use crate::sim::{Sample, SimInfo, Simulation, Summary, SCHEMA_VERSION};

/// One configuration, one scenario and the pass to simulate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunFile {
    pub config: ConfigSpec,
    pub scenario: ScenarioSpec,
    /// Which pass in the search window. Defaults to the first.
    #[serde(default)]
    pub pass: PassChoice,
}

/// A pass by its index in the search window (from 0), or by height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PassChoice {
    Index(usize),
    By(PassBy),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassBy {
    Highest,
    Lowest,
}

impl Default for PassChoice {
    fn default() -> PassChoice {
        PassChoice::Index(0)
    }
}

impl PassChoice {
    /// Read `3`, `highest` or `lowest`, as typed on the command line.
    pub fn parse(text: &str) -> Result<PassChoice, SimError> {
        match text {
            "highest" => Ok(PassChoice::By(PassBy::Highest)),
            "lowest" => Ok(PassChoice::By(PassBy::Lowest)),
            _ => text
                .parse()
                .map(PassChoice::Index)
                .map_err(|_| SimError::new(format!("a pass is a number, \"highest\" or \"lowest\", not \"{text}\""))),
        }
    }

    /// The index of the chosen pass in `list`.
    pub fn resolve(&self, list: &PassList) -> Result<usize, SimError> {
        let by_el = |a: &&crate::scenario::PassSummary, b: &&crate::scenario::PassSummary| a.max_el_deg.total_cmp(&b.max_el_deg);
        let found = match self {
            PassChoice::Index(i) => list.passes.get(*i),
            PassChoice::By(PassBy::Highest) => list.passes.iter().max_by(by_el),
            PassChoice::By(PassBy::Lowest) => list.passes.iter().min_by(by_el),
        };
        found.map(|p| p.index).ok_or_else(|| match self {
            PassChoice::Index(i) => {
                SimError::new(format!("there is no pass {} in the search window ({} found)", i, list.passes.len()))
            }
            PassChoice::By(_) => SimError::new("there are no passes in the search window"),
        })
    }
}

impl RunFile {
    pub fn from_yaml(text: &str) -> Result<RunFile, SimError> {
        serde_yaml::from_str(text).map_err(|e| SimError::new(format!("run file: {e}")))
    }

    /// Run the whole pass and return what was recorded.
    pub fn run(&self, presets: &Presets) -> Result<Trace, SimError> {
        let hw = presets.resolve(&self.config)?;
        let pass = self.pass.resolve(&find_passes(&self.scenario)?)?;
        let mut sim = Simulation::new(hw, &self.scenario, pass)?;
        let summary = sim.run_to_end()?;
        Ok(Trace { info: sim.info(), summary, samples: sim.samples().to_vec() })
    }
}

/// Everything one run recorded. Written as JSON, it can be read back for
/// replay or comparison; `info.schema_version` says which layout it uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub info: SimInfo,
    pub summary: Summary,
    pub samples: Vec<Sample>,
}

impl Trace {
    /// Read a trace, refusing one written by a newer layout.
    pub fn from_json(text: &str) -> Result<Trace, SimError> {
        #[derive(Deserialize)]
        struct Header {
            info: Version,
        }
        #[derive(Deserialize)]
        struct Version {
            schema_version: u32,
        }
        let header: Header = serde_json::from_str(text).map_err(|e| SimError::new(format!("trace: {e}")))?;
        if header.info.schema_version != SCHEMA_VERSION {
            return Err(SimError::new(format!(
                "trace: schema version {} cannot be read by this version, which reads {SCHEMA_VERSION}",
                header.info.schema_version
            )));
        }
        serde_json::from_str(text).map_err(|e| SimError::new(format!("trace: {e}")))
    }
}

/// The columns `write_csv` writes, in order. They are the fields of
/// `Sample`; a test checks that none is missing.
pub const CSV_COLUMNS: [&str; 21] = [
    "t_s",
    "utc",
    "target_az_deg",
    "target_el_deg",
    "boresight_az_deg",
    "boresight_el_deg",
    "err_x_arcsec",
    "err_y_arcsec",
    "err_arcsec",
    "in_fov",
    "axis1_deg",
    "axis2_deg",
    "axis1_rate_deg_s",
    "axis2_rate_deg_s",
    "axis1_cmd_rate_deg_s",
    "axis2_cmd_rate_deg_s",
    "axis1_accel_deg_s2",
    "axis2_accel_deg_s2",
    "rate_limited",
    "accel_limited",
    "lighting",
];

/// Samples as CSV, one row each, with a header row.
pub fn write_csv(samples: &[Sample], out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "{}", CSV_COLUMNS.join(","))?;
    for s in samples {
        let value = serde_json::to_value(s).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let cells: Vec<String> = CSV_COLUMNS
            .iter()
            .map(|c| match &value[*c] {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect();
        writeln!(out, "{}", cells.join(","))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::tests::iss_scenario;

    fn run_file(pass: PassChoice) -> RunFile {
        RunFile {
            config: ConfigSpec {
                name: "test".into(),
                telescope: "PlaneWave DeltaRho 350 (14\" f/3)".into(),
                camera: "Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO)".into(),
                mount: "PlaneWave L-350 (direct drive)".into(),
                mount_overrides: Default::default(),
            },
            scenario: iss_scenario(),
            pass,
        }
    }

    #[test]
    fn pass_choice_reads_numbers_and_names() {
        assert_eq!(PassChoice::parse("2").unwrap(), PassChoice::Index(2));
        assert_eq!(PassChoice::parse("highest").unwrap(), PassChoice::By(PassBy::Highest));
        assert!(PassChoice::parse("tallest").is_err());
        let yaml: RunFile = serde_yaml::from_str(&serde_yaml::to_string(&run_file(PassChoice::By(PassBy::Lowest))).unwrap()).unwrap();
        assert_eq!(yaml.pass, PassChoice::By(PassBy::Lowest));
    }

    #[test]
    fn pass_choice_finds_the_highest_and_rejects_a_missing_index() {
        let list = find_passes(&iss_scenario()).unwrap();
        let highest = PassChoice::By(PassBy::Highest).resolve(&list).unwrap();
        assert!(list.passes.iter().all(|p| p.max_el_deg <= list.passes[highest].max_el_deg));
        assert!(PassChoice::Index(99).resolve(&list).is_err());
    }

    #[test]
    fn a_trace_reads_back_and_a_newer_one_is_refused() {
        let trace = run_file(PassChoice::By(PassBy::Lowest)).run(&Presets::builtin().unwrap()).unwrap();
        assert!(trace.summary.complete);
        let json = serde_json::to_string(&trace).unwrap();
        assert_eq!(Trace::from_json(&json).unwrap(), trace);

        let newer = json.replacen(
            &format!("\"schema_version\":{SCHEMA_VERSION}"),
            &format!("\"schema_version\":{}", SCHEMA_VERSION + 1),
            1,
        );
        assert!(Trace::from_json(&newer).unwrap_err().0.contains("schema version"));
    }

    #[test]
    fn csv_has_every_sample_field() {
        let trace = run_file(PassChoice::By(PassBy::Lowest)).run(&Presets::builtin().unwrap()).unwrap();
        let fields = serde_json::to_value(&trace.samples[0]).unwrap();
        let mut names: Vec<&str> = fields.as_object().unwrap().keys().map(String::as_str).collect();
        let mut columns = CSV_COLUMNS.to_vec();
        names.sort_unstable();
        columns.sort_unstable();
        assert_eq!(names, columns);

        let mut out = Vec::new();
        write_csv(&trace.samples[..3], &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows.len(), 4);
        assert!(rows[0].starts_with("t_s,utc,"));
        assert_eq!(rows[1].split(',').count(), CSV_COLUMNS.len());
        assert!(rows[1].ends_with(",sunlit") || rows[1].ends_with(",penumbra") || rows[1].ends_with(",umbra"));
    }
}
