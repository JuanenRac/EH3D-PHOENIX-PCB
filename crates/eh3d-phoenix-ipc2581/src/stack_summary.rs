// SPDX-License-Identifier: GPL-3.0-or-later
//! The stack-up of a file read, as the layers of a board: copper, cores, prepregs and solder mask
//! from the top to the bottom, with the thickness of each in nanometres, the material and, when
//! the file gives them, the dielectric constant and the loss tangent.
//!
//! This is the bridge to the stack-up model of `eh3d-phoenix-stackup` (the command line makes the
//! `Stackup` and the `MaterialSample` of that crate from these rows). A value the file does not
//! give is left empty and reported (`PHOENIX-IPC2581-014`); nothing is made up.

use serde::Serialize;

use crate::codes::{Finding, Severity, READ_STACKUP_INCOMPLETE};
use crate::read_model::{IpcDoc, LayerFunction};

/// What a row of the stack-up is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StackKind {
    /// A copper layer (signal or plane).
    Copper,
    /// A rigid core.
    Core,
    /// A prepreg or bonding sheet.
    Prepreg,
    /// Solder mask.
    SolderMask,
}

impl StackKind {
    /// The word used by the reports and by `eh3d-phoenix-stackup`.
    pub fn word(self) -> &'static str {
        match self {
            StackKind::Copper => "copper",
            StackKind::Core => "core",
            StackKind::Prepreg => "prepreg",
            StackKind::SolderMask => "solder_mask",
        }
    }
}

/// One layer of the board.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StackRow {
    /// The place from the top, counted from 1.
    pub position: usize,
    /// The name of the layer.
    pub name: String,
    /// What it is.
    pub kind: StackKind,
    /// The thickness in nanometres, when the file gives it.
    pub thickness_nm: Option<i64>,
    /// The material, as the file names it; empty when it does not.
    pub material: String,
    /// The dielectric constant, when the file gives it.
    pub dk: Option<f64>,
    /// The loss tangent, when the file gives it.
    pub df: Option<f64>,
    /// The conductivity of a copper layer in siemens per metre, when the file gives it.
    pub conductivity_s_per_m: Option<f64>,
}

/// The stack-up of a board made from a file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StackSummary {
    /// The name of the stack-up in the file.
    pub name: String,
    /// The layers from the top to the bottom.
    pub rows: Vec<StackRow>,
    /// The sum of the thicknesses that are known, in nanometres.
    pub total_nm: i64,
    /// The thickness of the board that the file declares, in nanometres.
    pub declared_nm: Option<i64>,
    /// The layers of the stack-up that are not copper, dielectric or mask (legend, paste, ...) and
    /// are left out of the rows.
    pub left_out: usize,
    /// Whether every row has its thickness and every dielectric its constant and loss tangent.
    pub complete: bool,
    /// What is missing, one finding for each row.
    pub findings: Vec<Finding>,
}

/// The `index`-th stack-up of the file as a board; `None` when the file has no such stack-up.
pub fn stack_summary(doc: &IpcDoc, index: usize) -> Option<StackSummary> {
    let stackup = doc.stackups.get(index)?;
    let mut rows = Vec::new();
    let mut findings = Vec::new();
    let mut left_out = 0usize;
    let mut total = 0i64;
    for layer in &stackup.layers {
        let function = doc.layer(&layer.layer_ref).map(|l| &l.function);
        let kind = match function {
            Some(LayerFunction::Signal | LayerFunction::Plane) => StackKind::Copper,
            Some(LayerFunction::DielCore) => StackKind::Core,
            Some(LayerFunction::DielPreg) => StackKind::Prepreg,
            Some(LayerFunction::SolderMask) => StackKind::SolderMask,
            _ => {
                left_out += 1;
                continue;
            }
        };
        let spec = layer.spec.as_ref();
        let material = spec
            .and_then(|s| s.material.clone())
            .or_else(|| layer.material_type.clone())
            .unwrap_or_default();
        let row = StackRow {
            position: rows.len() + 1,
            name: layer.layer_ref.clone(),
            kind,
            thickness_nm: layer.thickness_nm,
            material,
            dk: spec.and_then(|s| s.dk),
            df: spec.and_then(|s| s.df),
            conductivity_s_per_m: spec.and_then(|s| s.conductivity_s_per_m),
        };
        let mut missing = Vec::new();
        match row.thickness_nm {
            Some(t) => total = total.saturating_add(t),
            None => missing.push("thickness"),
        }
        if matches!(kind, StackKind::Core | StackKind::Prepreg) {
            if row.dk.is_none() {
                missing.push("dielectric constant");
            }
            if row.df.is_none() {
                missing.push("loss tangent");
            }
        }
        if !missing.is_empty() {
            findings.push(Finding::new(
                READ_STACKUP_INCOMPLETE,
                Severity::Warning,
                Some(&row.name),
                format!("the file does not give the {}", missing.join(", the ")),
            ));
        }
        rows.push(row);
    }
    Some(StackSummary {
        name: stackup.name.clone(),
        complete: findings.is_empty() && !rows.is_empty(),
        rows,
        total_nm: total,
        declared_nm: stackup.overall_thickness_nm,
        left_out,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::{read_str, ReadOptions};

    const FILE: &str = r#"<?xml version="1.0"?>
<IPC-2581 revision="B">
 <Ecad name="d">
  <CadHeader units="MICRON">
   <Spec name="M"><General type="MATERIAL"><Property text="Mask"/></General><Dielectric type="DIELECTRIC_CONSTANT"><Property value="3.5"/></Dielectric></Spec>
   <Spec name="C"><General type="MATERIAL"><Property text="Foil"/></General><Conductor type="CONDUCTIVITY"><Property value="595900" unit="MHO/CM"/></Conductor></Spec>
   <Spec name="P"><General type="MATERIAL"><Property text="Prepreg A"/></General><Dielectric type="DIELECTRIC_CONSTANT"><Property value="4.2"/></Dielectric><Dielectric type="LOSS_TANGENT"><Property value="0.02"/></Dielectric></Spec>
   <Spec name="K"><General type="MATERIAL"><Property text="Core B"/></General><Dielectric type="DIELECTRIC_CONSTANT"><Property value="4.6"/></Dielectric></Spec>
  </CadHeader>
  <CadData>
   <Layer name="MT" layerFunction="SOLDERMASK" side="TOP" polarity="POSITIVE"/>
   <Layer name="L1" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/>
   <Layer name="D1" layerFunction="DIELPREG" side="INTERNAL" polarity="POSITIVE"/>
   <Layer name="L2" layerFunction="PLANE" side="INTERNAL" polarity="NEGATIVE"/>
   <Layer name="D2" layerFunction="DIELCORE" side="INTERNAL" polarity="POSITIVE"/>
   <Layer name="L3" layerFunction="SIGNAL" side="BOTTOM" polarity="POSITIVE"/>
   <Layer name="SS" layerFunction="SILKSCREEN" side="TOP" polarity="POSITIVE"/>
   <Stackup name="S" overallThickness="1600" tolPlus="100" tolMinus="100" whereMeasured="METAL">
    <StackupGroup name="G" thickness="1600">
     <StackupLayer layerOrGroupRef="MT" thickness="20" sequence="1"><SpecRef id="M"/></StackupLayer>
     <StackupLayer layerOrGroupRef="L1" thickness="35" sequence="2"><SpecRef id="C"/></StackupLayer>
     <StackupLayer layerOrGroupRef="D1" thickness="200.5" sequence="3"><SpecRef id="P"/></StackupLayer>
     <StackupLayer layerOrGroupRef="L2" thickness="35" sequence="4"><SpecRef id="C"/></StackupLayer>
     <StackupLayer layerOrGroupRef="D2" thickness="1200" sequence="5"><SpecRef id="K"/></StackupLayer>
     <StackupLayer layerOrGroupRef="L3" sequence="6"><SpecRef id="C"/></StackupLayer>
     <StackupLayer layerOrGroupRef="SS" thickness="10" sequence="7"/>
     <StackupLayer layerOrGroupRef="G" thickness="1" sequence="8"/>
    </StackupGroup>
   </Stackup>
  </CadData>
 </Ecad>
</IPC-2581>"#;

    #[test]
    fn the_rows_follow_the_file_with_kinds_materials_and_exact_thicknesses() {
        let doc = read_str(FILE, &ReadOptions::default()).unwrap();
        let s = stack_summary(&doc, 0).unwrap();
        assert_eq!(s.name, "S");
        assert_eq!(s.declared_nm, Some(1_600_000));
        let kinds: Vec<&str> = s.rows.iter().map(|r| r.kind.word()).collect();
        assert_eq!(
            kinds,
            [
                "solder_mask",
                "copper",
                "prepreg",
                "copper",
                "core",
                "copper"
            ]
        );
        assert_eq!(s.rows[2].thickness_nm, Some(200_500));
        assert_eq!(s.rows[2].material, "Prepreg A");
        assert_eq!((s.rows[2].dk, s.rows[2].df), (Some(4.2), Some(0.02)));
        assert_eq!(s.rows[1].conductivity_s_per_m, Some(59_590_000.0));
        assert_eq!(s.rows[5].thickness_nm, None);
        assert_eq!(s.total_nm, 20_000 + 35_000 + 200_500 + 35_000 + 1_200_000);
        // the legend layer is left out, and the reference to the group is not a layer
        assert_eq!(s.left_out, 1);
        assert_eq!(s.rows.len(), 6);
    }

    #[test]
    fn what_the_file_does_not_give_is_reported_and_not_made_up() {
        let doc = read_str(FILE, &ReadOptions::default()).unwrap();
        let s = stack_summary(&doc, 0).unwrap();
        assert!(!s.complete);
        let subjects: Vec<&str> = s
            .findings
            .iter()
            .map(|f| f.subject.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(subjects, ["D2", "L3"]);
        assert!(s.findings[0].detail.contains("loss tangent"));
        assert!(s.findings[1].detail.contains("thickness"));
        assert!(stack_summary(&doc, 1).is_none());
    }
}
