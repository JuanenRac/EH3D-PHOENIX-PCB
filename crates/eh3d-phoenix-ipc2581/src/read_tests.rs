// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the reader of IPC-2581 files. The files are small ones written for these tests in the
//! shape that files of this format have; none of them is a copy of a file of a tool.

use crate::codes::{READ_LIMIT, READ_NOT_IPC2581, READ_NOT_XML, READ_SKIPPED, READ_VALUE_INVALID};
use crate::read::{read_bytes, read_str, ReadOptions};
use crate::read_model::{LayerFunction, Plating, Side};
use crate::units::Unit;

const REV_C: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<IPC-2581 revision="C" xmlns="http://webstds.ipc.org/2581" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
 <Content roleRef="Owner">
  <FunctionMode mode="FULL" level="1"/>
  <StepRef name="board"/>
  <LayerRef name="TOP"/>
  <DictionaryLineDesc units="MILLIMETER">
   <EntryLineDesc id="R1"><LineDesc lineEnd="ROUND" lineWidth="0.25"/></EntryLineDesc>
   <EntryLineDesc id="R2"><LineDesc lineEnd="ROUND" lineWidth="0.0005"/></EntryLineDesc>
  </DictionaryLineDesc>
  <DictionaryStandard units="MILLIMETER">
   <EntryStandard id="C1"><Circle diameter="0.6"/></EntryStandard>
   <EntryStandard id="C2"><Circle diameter="1.2"/></EntryStandard>
  </DictionaryStandard>
 </Content>
 <LogisticHeader><Role id="Owner"/></LogisticHeader>
 <HistoryRecord number="1" origination="2020-01-01T00:00:00" software="A Tool 1.0"><FileRevision fileRevisionId="1"/></HistoryRecord>
 <Bom name="BOM_1">
  <BomHeader assembly="a" revision="1"/>
  <BomItem OEMDesignNumberRef="R-10K" quantity="2" pinCount="2" category="ELECTRICAL">
   <RefDes name="R1" packageRef="0603" populate="true" layerRef="TOP"/>
   <RefDes name="R2" packageRef="0603" populate="true" layerRef="TOP"/>
   <Characteristics category="ELECTRICAL"/>
  </BomItem>
  <BomItem OEMDesignNumberRef="MOUNT" quantity="x" category="MECHANICAL"/>
 </Bom>
 <Ecad name="board">
  <CadHeader units="MILLIMETER">
   <Spec name="FR4"><General type="MATERIAL"><Property text="FR-4"/></General><Dielectric type="DIELECTRIC_CONSTANT"><Property value="4.3"/></Dielectric><Dielectric type="LOSS_TANGENT"><Property value="0.019"/></Dielectric><Dielectric type="RESIN_CONTENT"><Property value="48" unit="PERCENT"/></Dielectric></Spec>
   <Spec name="CU"><General type="MATERIAL"><Property text="Copper"/></General><Conductor type="CONDUCTIVITY"><Property value="595900" unit="MHO/CM"/></Conductor></Spec>
  </CadHeader>
  <CadData>
   <Layer name="TOP" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/>
   <Layer name="CORE" layerFunction="DIELCORE" side="INTERNAL" polarity="POSITIVE"/>
   <Layer name="BOT" layerFunction="SIGNAL" side="BOTTOM" polarity="POSITIVE"/>
   <Layer name="PWR" layerFunction="PLANE" side="INTERNAL" polarity="NEGATIVE"/>
   <Layer name="DRILL_1_2" layerFunction="DRILL" side="ALL" polarity="POSITIVE"><Span fromLayer="TOP" toLayer="BOT"/></Layer>
   <Stackup name="S4" overallThickness="1.6" tolPlus="0.1" tolMinus="0.1" whereMeasured="METAL">
    <StackupGroup name="ALL" thickness="1.6">
     <StackupLayer layerOrGroupRef="TOP" thickness="0.035" sequence="1"><SpecRef id="CU"/></StackupLayer>
     <StackupLayer layerOrGroupRef="CORE" thickness="1.53" tolPlus="0.05" tolMinus="0.05" sequence="2" materialType="FR-4"><SpecRef id="FR4"/></StackupLayer>
     <StackupLayer layerOrGroupRef="BOT" thickness="0.035" sequence="3"><SpecRef id="CU"/></StackupLayer>
    </StackupGroup>
   </Stackup>
   <Step name="board" type="BOARD">
    <Datum x="1.5" y="-2.25"/>
    <Profile>
     <Polygon><PolyBegin x="0" y="0"/><PolyStepSegment x="40" y="0"/><PolyStepCurve x="50" y="10" centerX="40" centerY="10" clockwise="false"/><PolyStepSegment x="50" y="30"/><PolyStepSegment x="0" y="30"/><PolyStepSegment x="0" y="0"/></Polygon>
     <Cutout><PolyBegin x="10" y="10"/><PolyStepSegment x="12" y="10"/><PolyStepSegment x="12" y="12"/><PolyStepSegment x="10" y="10"/></Cutout>
    </Profile>
    <PadStackDef name="PS1"><PadstackHoleDef name="PH1" diameter="0.3" platingStatus="PLATED" plusTol="0.05" minusTol="0.05" x="0" y="0"/><PadstackPadDef layerRef="TOP" padUse="REGULAR"><Location x="0" y="0"/><StandardPrimitiveRef id="C1"/></PadstackPadDef><PadstackPadDef layerRef="BOT" padUse="REGULAR"><StandardPrimitiveRef id="C1"/></PadstackPadDef></PadStackDef>
    <Package name="0603" type="CHIP" height="0.45"><Outline><Polygon><PolyBegin x="0" y="0"/></Polygon></Outline><Pin number="1"/><Pin number="2"/></Package>
    <Component refDes="R1" packageRef="0603" layerRef="TOP" part="R-10K" mountType="SMT"><Xform rotation="90" mirror="false"/><Location x="10.5" y="20"/></Component>
    <Component refDes="R2" packageRef="0603" layerRef="BOT" part="R-10K" mountType="SMT"><Xform rotation="270.5" mirror="true"/><Location x="12.5" y="20"/></Component>
    <LogicalNet name="VCC" netClass="POWER"><PinRef componentRef="R1" pin="1"/><PinRef componentRef="R2" pin="1"/></LogicalNet>
    <LogicalNet name="GND" netClass="GROUND"><PinRef componentRef="R1" pin="2"/></LogicalNet>
    <PhyNetGroup name="AllPhyNets"><PhyNet name="VCC"><PhyNetPoint x="1" y="1" layerRef="TOP" netNode="END" exposure="EXPOSED"/><PhyNetPoint x="2" y="1" layerRef="TOP" netNode="END" exposure="EXPOSED"/></PhyNet></PhyNetGroup>
    <LayerFeature layerRef="TOP">
     <Set net="VCC">
      <Pad padstackDefRef="PS1"><Location x="10" y="20"/><PinRef componentRef="R1" pin="1"/></Pad>
      <Features><Location x="0" y="0"/><Line startX="1" startY="2" endX="3" endY="2"><LineDescRef id="R1"/></Line></Features>
      <Features><Location x="1" y="1"/><Line startX="0" startY="0" endX="2" endY="0"><LineDesc lineEnd="ROUND" lineWidth="0.1"/></Line></Features>
      <Features><Location x="0" y="0"/><Arc startX="0" startY="0" endX="2" endY="2" centerX="2" centerY="0" clockwise="true"><LineDescRef id="R2"/></Arc></Features>
      <Features><Location x="0" y="0"/><Polyline><PolyBegin x="0" y="0"/><PolyStepSegment x="5" y="0"/><PolyStepCurve x="6" y="1" centerX="5" centerY="1" clockwise="false"/><LineDescRef id="R1"/></Polyline></Features>
     </Set>
     <Set net="GND">
      <Features><Location x="0" y="0"/><Contour><Polygon><PolyBegin x="0" y="0"/><PolyStepSegment x="9" y="0"/><PolyStepSegment x="9" y="9"/><PolyStepSegment x="0" y="0"/></Polygon><Cutout><PolyBegin x="1" y="1"/><PolyStepSegment x="2" y="1"/><PolyStepSegment x="2" y="2"/><PolyStepSegment x="1" y="1"/></Cutout></Contour></Features>
      <NonstandardAttribute name="x" value="1" type="STRING"/>
     </Set>
    </LayerFeature>
    <LayerFeature layerRef="DRILL_1_2">
     <Set>
      <Hole name="H1" diameter="0.3" platingStatus="VIA" plusTol="0.05" minusTol="0.05" x="5" y="6"/>
      <Hole name="H2" diameter="3.2" platingStatus="NONPLATED" plusTol="0" minusTol="0" x="45" y="25"/>
      <SlotCavity name="S1" platingStatus="PLATED" plusTol="0" minusTol="0"><Location x="1" y="1"/></SlotCavity>
     </Set>
    </LayerFeature>
   </Step>
  </CadData>
 </Ecad>
</IPC-2581>"#;

fn options() -> ReadOptions {
    ReadOptions::default()
}

#[test]
fn a_revision_c_file_gives_its_layers_stackup_and_specifications() {
    let doc = read_str(REV_C, &options()).unwrap();
    assert_eq!(doc.revision, "C");
    assert_eq!(doc.units, Some(Unit::Millimeter));
    assert_eq!(doc.modes, ["FULL"]);
    assert_eq!(doc.software, "A Tool 1.0");
    assert_eq!(doc.layers.len(), 5);
    let top = doc.layer("TOP").unwrap();
    assert_eq!(top.function, LayerFunction::Signal);
    assert_eq!(top.side, Side::Top);
    assert!(!top.negative);
    let pwr = doc.layer("PWR").unwrap();
    assert_eq!(pwr.function, LayerFunction::Plane);
    assert!(pwr.negative);
    assert_eq!(pwr.side, Side::Internal);
    let drill = doc.layer("DRILL_1_2").unwrap();
    assert_eq!(drill.span, Some(("TOP".to_owned(), "BOT".to_owned())));
    assert_eq!(drill.side, Side::Both);
    let s = &doc.stackups[0];
    assert_eq!(s.name, "S4");
    assert_eq!(s.overall_thickness_nm, Some(1_600_000));
    assert_eq!(s.tol_plus_nm, Some(100_000));
    assert_eq!(s.where_measured, "METAL");
    assert_eq!(s.layers.len(), 3);
    let core = &s.layers[1];
    assert_eq!(core.thickness_nm, Some(1_530_000));
    assert_eq!(core.tol_plus_nm, Some(50_000));
    assert_eq!(core.sequence, Some(2));
    assert_eq!(core.group, "ALL");
    assert_eq!(core.material_type.as_deref(), Some("FR-4"));
    let spec = core.spec.as_ref().unwrap();
    assert_eq!(spec.material.as_deref(), Some("FR-4"));
    assert_eq!(spec.dk, Some(4.3));
    assert_eq!(spec.df, Some(0.019));
    assert_eq!(spec.resin_percent, Some(48.0));
    let copper = s.layers[0].spec.as_ref().unwrap();
    assert_eq!(copper.conductivity_s_per_m, Some(59_590_000.0));
    assert_eq!(doc.specs.len(), 2);
}

#[test]
fn a_revision_c_file_gives_the_outline_holes_parts_nets_and_copper() {
    let doc = read_str(REV_C, &options()).unwrap();
    assert_eq!(doc.steps.len(), 1);
    let step = &doc.steps[0];
    assert_eq!(step.name, "board");
    assert_eq!(step.datum_nm, Some((1_500_000, -2_250_000)));
    let outline = step.profile.as_ref().unwrap();
    assert_eq!(outline.outer.len(), 6);
    assert_eq!(outline.outer[2].center_nm, Some((40_000_000, 10_000_000)));
    assert!(!outline.outer[2].clockwise);
    assert_eq!(outline.cutouts.len(), 1);
    assert_eq!(outline.cutouts[0].len(), 4);
    assert_eq!(step.padstacks.len(), 1);
    assert_eq!(step.padstacks[0].holes[0].diameter_nm, 300_000);
    assert_eq!(step.padstacks[0].pads.len(), 2);
    // the holes of the drill layer
    assert_eq!(step.holes.len(), 2);
    assert_eq!(step.holes[0].plating, Plating::Via);
    assert_eq!(step.holes[0].layer, "DRILL_1_2");
    assert_eq!(
        (step.holes[0].x_nm, step.holes[0].y_nm),
        (5_000_000, 6_000_000)
    );
    assert_eq!(step.holes[0].plus_tol_nm, 50_000);
    assert_eq!(step.holes[1].plating, Plating::NonPlated);
    assert_eq!(step.holes[1].diameter_nm, 3_200_000);
    assert_eq!(step.packages.len(), 1);
    assert_eq!(step.packages[0].pins, 2);
    assert_eq!(step.packages[0].height_nm, Some(450_000));
    assert_eq!(step.components.len(), 2);
    let r2 = &step.components[1];
    assert_eq!(r2.ref_des, "R2");
    assert_eq!((r2.x_nm, r2.y_nm), (12_500_000, 20_000_000));
    assert_eq!(r2.rotation_deg, 270.5);
    assert!(r2.mirror);
    assert_eq!(r2.layer, "BOT");
    assert_eq!(step.nets.len(), 2);
    assert_eq!(step.nets[0].name, "VCC");
    assert_eq!(step.nets[0].class, "POWER");
    assert_eq!(
        step.nets[0].pins,
        [("R1".into(), "1".into()), ("R2".into(), "1".into())]
    );
    assert_eq!(step.phy_nets[0].points, 2);
    let top = step.contents.iter().find(|c| c.layer == "TOP").unwrap();
    assert_eq!(
        (top.pads, top.lines, top.arcs, top.polylines, top.regions),
        (1, 2, 1, 1, 1)
    );
    // a line with a width from the dictionary, one with its own width and a placement offset
    assert_eq!(top.line_items[0].width_nm, 250_000);
    assert_eq!(top.line_items[0].net, "VCC");
    assert_eq!(top.line_items[1].from_nm, (1_000_000, 1_000_000));
    assert_eq!(top.line_items[1].to_nm, (3_000_000, 1_000_000));
    assert_eq!(top.line_items[1].width_nm, 100_000);
    assert_eq!(top.arc_items[0].center_nm, (2_000_000, 0));
    assert!(top.arc_items[0].clockwise);
    assert_eq!(top.arc_items[0].width_nm, 500);
    assert_eq!(top.polyline_items[0].points.len(), 3);
    assert_eq!(
        top.polyline_items[0].points[2].center_nm,
        Some((5_000_000, 1_000_000))
    );
    assert_eq!(top.region_items[0].net, "GND");
    assert_eq!(top.region_items[0].cutouts.len(), 1);
    let drill = step
        .contents
        .iter()
        .find(|c| c.layer == "DRILL_1_2")
        .unwrap();
    assert_eq!((drill.holes, drill.slots), (2, 1));
    let c = doc.counts();
    assert_eq!((c.holes, c.holes_plated, c.holes_non_plated), (2, 1, 1));
    assert_eq!((c.components, c.nets, c.net_pins, c.phy_nets), (2, 2, 3, 1));
    assert_eq!((c.layers, c.copper_layers, c.stackup_layers), (5, 3, 3));
}

#[test]
fn the_bill_of_materials_and_the_dictionaries_are_read() {
    let doc = read_str(REV_C, &options()).unwrap();
    assert_eq!(doc.boms.len(), 1);
    let bom = &doc.boms[0];
    assert_eq!(bom.name, "BOM_1");
    assert_eq!(bom.items.len(), 2);
    assert_eq!(bom.items[0].part, "R-10K");
    assert_eq!(bom.items[0].quantity, Some(2));
    assert_eq!(bom.items[0].ref_des, ["R1", "R2"]);
    // a quantity that is not a number is left out and told
    assert_eq!(bom.items[1].quantity, None);
    assert!(doc
        .findings
        .iter()
        .any(|f| f.code == READ_VALUE_INVALID && f.subject.as_deref() == Some("BomItem@quantity")));
    let std = doc
        .dictionaries
        .iter()
        .find(|d| d.name == "DictionaryStandard")
        .unwrap();
    assert_eq!((std.unit, std.elements), (Some(Unit::Millimeter), 5));
}

#[test]
fn what_the_reader_does_not_read_is_named_with_its_count() {
    let doc = read_str(REV_C, &options()).unwrap();
    assert_eq!(doc.skipped.get("NonstandardAttribute"), Some(&1));
    let f = doc
        .findings
        .iter()
        .find(|f| f.code == READ_SKIPPED && f.subject.as_deref() == Some("NonstandardAttribute"))
        .unwrap();
    assert_eq!(f.detail, "1 times");
    // the elements of the header and the content that carry nothing the reader uses are quiet
    assert!(!doc.skipped.contains_key("LogisticHeader"));
    assert!(!doc.skipped.contains_key("StepRef"));
}

#[test]
fn every_unit_is_converted_exactly() {
    let text = |unit: &str, value: &str| {
        format!(
            "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"{unit}\"/><CadData><Step name=\"s\"><LayerFeature layerRef=\"D\"><Set><Hole name=\"h\" diameter=\"{value}\" platingStatus=\"PLATED\" x=\"{value}\" y=\"0\"/></Set></LayerFeature></Step></CadData></Ecad></IPC-2581>"
        )
    };
    let nm = |unit: &str, value: &str| {
        let doc = read_str(&text(unit, value), &options()).unwrap();
        doc.steps[0].holes[0].diameter_nm
    };
    assert_eq!(nm("MILLIMETER", "0.3"), 300_000);
    assert_eq!(nm("INCH", "0.0118"), 299_720);
    assert_eq!(nm("MICRON", "300"), 300_000);
    assert_eq!(nm("MIL", "11.8"), 299_720);
    assert_eq!(nm("inch", "0.0254"), 645_160);
}

#[test]
fn a_file_without_a_unit_assumes_millimetres_and_says_so() {
    let doc = read_str(
        "<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadData><Layer name=\"L\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/></CadData></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    assert_eq!(doc.units, None);
    assert!(doc
        .findings
        .iter()
        .any(|f| f.subject.as_deref() == Some("CadHeader@units")));
    let odd = read_str(
        "<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"FURLONG\"/></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    assert_eq!(odd.units, None);
    assert!(odd
        .findings
        .iter()
        .any(|f| f.subject.as_deref() == Some("CadHeader@units")));
}

#[test]
fn a_value_that_is_not_a_length_is_left_out_with_its_attribute_named() {
    let doc = read_str(
        "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><Step name=\"s\"><Datum x=\"abc\" y=\"1\"/><LayerFeature layerRef=\"D\"><Set><Hole name=\"a\" diameter=\"1e999\" x=\"0\" y=\"0\"/><Hole name=\"b\" diameter=\"0.01\" x=\"0,5\" y=\"0\"/><Hole name=\"c\" x=\"0\" y=\"0\"/></Set></LayerFeature></Step></CadData></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    let step = &doc.steps[0];
    assert_eq!(step.datum_nm, None);
    // the hole with a good diameter is kept, with the place it could read
    assert_eq!(step.holes.len(), 1);
    assert_eq!(step.holes[0].name, "b");
    assert_eq!(step.holes[0].x_nm, 0);
    let subjects: Vec<&str> = doc
        .findings
        .iter()
        .filter(|f| f.code == READ_VALUE_INVALID)
        .filter_map(|f| f.subject.as_deref())
        .collect();
    assert!(subjects.contains(&"Datum@x,y"), "{subjects:?}");
    assert!(subjects.contains(&"Hole@diameter"), "{subjects:?}");
    assert!(subjects.contains(&"Hole@x"), "{subjects:?}");
}

#[test]
fn the_older_form_with_step_lists_and_old_padstacks_is_read() {
    let doc = read_str(
        r#"<IPC-2581 xmlns="http://webstds.ipc.org/2581">
<Content><FunctionMode mode="DESIGN" level="1"/><StepListRef>b</StepListRef><LayerDescRef>b</LayerDescRef></Content>
<Ecad name="b"><CadHeader units="MILLIMETER"/><CadData>
<LayerDesc name="b"><Layer name="TOP" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/><Layer name="D" layerFunction="DRILL" side="ALL" polarity="POSITIVE"><Span fromLayer="TOP" toLayer="TOP"/></Layer>
<Stackup name="s" overallThickness="1.6"><StackupGroup name="g"><StackupLayer layerOrGroupRef="TOP" thickness="0.035" sequence="1"/></StackupGroup></Stackup></LayerDesc>
<StepList name="b"><Step name="b"><Datum x="0" y="0"/>
<PadStack net="N"><LayerHole name="LH1" diameter="0.4" x="1" y="2" platingStatus="PLATED"/><LayerPad layerRef="TOP" pin="1"/><LayerPad layerRef="TOP" pin="2"/></PadStack>
<Component refDes="U1" packageRef="P" layerRef="TOP"><Xform rotation="0"/><Location x="1" y="1"/></Component>
<LogicalNet name="N"><LogicalNetPin componentRef="U1" pin="1"/><LogicalNetPin componentRef="U1" pin="2"/></LogicalNet>
</Step></StepList></CadData></Ecad></IPC-2581>"#,
        &options(),
    )
    .unwrap();
    assert_eq!(doc.revision, "");
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.stackups[0].layers.len(), 1);
    let c = doc.counts();
    assert_eq!(
        (c.steps, c.holes, c.pads, c.components, c.nets, c.net_pins),
        (1, 1, 2, 1, 1, 2)
    );
    assert_eq!(doc.steps[0].holes[0].diameter_nm, 400_000);
}

#[test]
fn a_document_that_is_not_ipc2581_or_not_xml_is_refused_with_its_code() {
    let e = read_str("<other/>", &options()).unwrap_err();
    assert_eq!(e.code, READ_NOT_IPC2581);
    assert!(e.detail.contains("other"), "{}", e.detail);
    for text in [
        "",
        "not xml",
        "<IPC-2581>",
        "<IPC-2581></Other>",
        "<IPC-2581 a=1/>",
    ] {
        let e = read_str(text, &options()).unwrap_err();
        assert_eq!(e.code, READ_NOT_XML, "{text}");
        assert!(e.detail.contains("malformed XML"), "{text}: {}", e.detail);
    }
}

#[test]
fn a_name_from_the_file_cannot_smuggle_control_characters_into_a_report() {
    let doc = read_str(
        "<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><Layer name=\"A&#27;[31mB\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/></CadData><\u{1b}x/></Ecad></IPC-2581>",
        &options(),
    );
    // an escape in a tag name is not well formed; the one in a value is cleaned
    assert!(doc.is_err());
    let doc = read_str(
        "<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><Layer name=\"A&#27;[31mB\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/></CadData></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    assert_eq!(doc.layers[0].name, "A?[31mB");
    let long = "x".repeat(500);
    let doc = read_str(
        &format!("<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><{long}/></CadData></Ecad></IPC-2581>"),
        &options(),
    )
    .unwrap();
    assert_eq!(doc.skipped.keys().next().unwrap().chars().count(), 80);
}

#[test]
fn the_limits_refuse_a_file_with_the_limit_code() {
    let big = ReadOptions {
        max_bytes: 100,
        ..options()
    };
    assert_eq!(read_str(REV_C, &big).unwrap_err().code, READ_LIMIT);
    assert_eq!(
        read_bytes(REV_C.as_bytes(), &big).unwrap_err().code,
        READ_LIMIT
    );
    let deep = ReadOptions {
        max_depth: 5,
        ..options()
    };
    assert_eq!(read_str(REV_C, &deep).unwrap_err().code, READ_LIMIT);
    let few = ReadOptions {
        max_elements: 20,
        ..options()
    };
    assert_eq!(read_str(REV_C, &few).unwrap_err().code, READ_LIMIT);
    let items = ReadOptions {
        max_items: 3,
        ..options()
    };
    let e = read_str(REV_C, &items).unwrap_err();
    assert_eq!(e.code, READ_LIMIT);
    assert!(e.detail.contains("items"), "{}", e.detail);
}

#[test]
fn the_geometry_is_kept_only_when_asked_but_the_counts_always() {
    let lean = ReadOptions {
        keep_geometry: false,
        ..options()
    };
    let doc = read_str(REV_C, &lean).unwrap();
    let top = doc.steps[0]
        .contents
        .iter()
        .find(|c| c.layer == "TOP")
        .unwrap();
    assert_eq!(
        (top.lines, top.arcs, top.polylines, top.regions),
        (2, 1, 1, 1)
    );
    assert!(top.line_items.is_empty() && top.arc_items.is_empty());
    assert!(top.polyline_items.is_empty() && top.region_items.is_empty());
    let full = read_str(REV_C, &options()).unwrap();
    assert_eq!(full.counts(), doc.counts());
}

#[test]
fn bytes_that_are_not_utf8_are_read_as_latin1_and_a_bom_is_dropped() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(
        b"<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><Layer name=\"Caf\xe9\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/></CadData></Ecad></IPC-2581>",
    );
    let doc = read_bytes(&bytes, &options()).unwrap();
    assert!(doc.layers[0].name.starts_with("Caf"));
    assert_eq!(doc.findings[0].code, READ_VALUE_INVALID);
    assert_eq!(doc.findings[0].subject.as_deref(), Some("encoding"));
    let good = read_bytes(REV_C.as_bytes(), &options()).unwrap();
    assert!(good
        .findings
        .iter()
        .all(|f| f.subject.as_deref() != Some("encoding")));
}

#[test]
fn the_same_file_always_gives_the_same_result() {
    let a = read_str(REV_C, &options()).unwrap();
    let b = read_str(REV_C, &options()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_stackup_group_inside_a_group_and_a_reference_to_a_group_are_flattened() {
    let doc = read_str(
        "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"MICRON\"/><CadData><Layer name=\"A\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/><Layer name=\"B\" layerFunction=\"DIELCORE\" side=\"INTERNAL\" polarity=\"POSITIVE\"/><Stackup name=\"s\" overallThickness=\"100\"><StackupGroup name=\"outer\"><StackupLayer layerOrGroupRef=\"inner\" thickness=\"1\"/><StackupGroup name=\"inner\"><StackupLayer layerOrGroupRef=\"A\" thickness=\"35\"/><StackupLayer layerOrGroupRef=\"B\" thickness=\"65\"/></StackupGroup></StackupGroup></Stackup></CadData></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    let layers = &doc.stackups[0].layers;
    assert_eq!(layers.len(), 2);
    assert_eq!(
        (layers[0].layer_ref.as_str(), layers[0].group.as_str()),
        ("A", "inner")
    );
    assert_eq!(layers[1].thickness_nm, Some(65_000));
}

#[test]
fn a_file_with_thousands_of_items_is_counted_exactly() {
    let mut text = String::from(
        "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"MILLIMETER\"/><CadData><Step name=\"s\">",
    );
    for i in 0..3000 {
        text.push_str(&format!(
            "<Component refDes=\"C{i}\" packageRef=\"P\" layerRef=\"TOP\"><Location x=\"{i}\" y=\"-{i}.5\"/></Component>"
        ));
    }
    text.push_str("<LayerFeature layerRef=\"D\"><Set>");
    for i in 0..2500 {
        text.push_str(&format!(
            "<Hole name=\"H{i}\" diameter=\"0.{:03}\" platingStatus=\"PLATED\" x=\"{i}\" y=\"0\"/>",
            100 + i % 800
        ));
    }
    text.push_str("</Set></LayerFeature></Step></CadData></Ecad></IPC-2581>");
    let doc = read_str(&text, &options()).unwrap();
    let c = doc.counts();
    assert_eq!((c.components, c.holes, c.holes_plated), (3000, 2500, 2500));
    assert_eq!(doc.steps[0].components[2999].y_nm, -2_999_500_000);
    assert_eq!(
        doc.steps[0].holes[2499].diameter_nm,
        (100 + 2499 % 800) * 1000
    );
    assert_eq!(doc.steps[0].contents[0].holes, 2500);
}

#[test]
fn a_step_that_is_empty_or_repeated_is_kept_as_it_is() {
    let doc = read_str(
        "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/><CadData><Step name=\"a\"/><Step name=\"b\"><Datum x=\"1\" y=\"1\"/></Step></CadData></Ecad></IPC-2581>",
        &options(),
    )
    .unwrap();
    let names: Vec<&str> = doc.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(doc.steps[1].datum_nm, Some((25_400_000, 25_400_000)));
}

#[test]
fn mutated_files_never_panic_with_a_fixed_seed() {
    let base = REV_C.as_bytes();
    let mut state: u64 = 0x2581_c0de;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    let opts = ReadOptions {
        max_items: 10_000,
        ..options()
    };
    for _ in 0..1500 {
        let mut bytes = base.to_vec();
        for _ in 0..(1 + next() % 6) {
            let at = next() % bytes.len();
            match next() % 4 {
                0 => bytes[at] = b"<>/=\"' 0123456789.-eE&;!"[next() % 24],
                1 => {
                    bytes.remove(at);
                }
                2 => bytes.insert(at, b"<>/=\"'&"[next() % 7]),
                _ => {
                    let end = (at + 1 + next() % 40).min(bytes.len());
                    bytes.drain(at..end);
                }
            }
        }
        let _ = read_bytes(&bytes, &opts);
    }
}

fn top_after_replacing(from: &str, to: &str) -> crate::read_model::LayerContent {
    assert!(REV_C.contains(from), "{from}");
    let text = REV_C.replace(from, to);
    let doc = read_str(&text, &options()).unwrap();
    doc.steps[0]
        .contents
        .iter()
        .find(|c| c.layer == "TOP")
        .unwrap()
        .clone()
}

#[test]
fn a_quarter_turn_of_a_group_of_features_moves_its_points() {
    let top = top_after_replacing(
        r#"<Features><Location x="1" y="1"/><Line startX="0" startY="0" endX="2" endY="0">"#,
        r#"<Features><Xform rotation="90"/><Location x="1" y="1"/><Line startX="0" startY="0" endX="2" endY="0">"#,
    );
    // (0,0) -> (0,0) and (2,0) -> (0,2), then the location (1,1)
    assert_eq!(top.line_items[1].from_nm, (1_000_000, 1_000_000));
    assert_eq!(top.line_items[1].to_nm, (1_000_000, 3_000_000));
    assert_eq!(top.placed_features, 1);
}

#[test]
fn a_mirror_changes_the_side_of_x_and_the_direction_of_an_arc() {
    let top = top_after_replacing(
        r#"<Features><Location x="0" y="0"/><Arc startX="0" startY="0" endX="2" endY="2" centerX="2" centerY="0" clockwise="true">"#,
        r#"<Features><Xform mirror="true"/><Location x="0" y="0"/><Arc startX="0" startY="0" endX="2" endY="2" centerX="2" centerY="0" clockwise="true">"#,
    );
    let arc = &top.arc_items[0];
    assert_eq!(arc.to_nm, (-2_000_000, 2_000_000));
    assert_eq!(arc.center_nm, (-2_000_000, 0));
    assert!(!arc.clockwise);
}

#[test]
fn the_turn_comes_after_the_mirror_and_before_the_move_in_polylines_and_regions() {
    let top = top_after_replacing(
        r#"<Features><Location x="0" y="0"/><Polyline><PolyBegin x="0" y="0"/>"#,
        r#"<Features><Xform rotation="180" mirror="true" xOffset="1" yOffset="0"/><Location x="0" y="1"/><Polyline><PolyBegin x="0" y="0"/>"#,
    );
    let poly = &top.polyline_items[0];
    // mirror (5,0) -> (-5,0), turn 180 -> (5,0), move (1,1) -> (6,1)
    assert_eq!(
        (poly.points[1].x_nm, poly.points[1].y_nm),
        (6_000_000, 1_000_000)
    );
    // the curve ran counter-clockwise and the mirror makes it clockwise
    assert!(poly.points[2].clockwise);
}

#[test]
fn a_scale_and_a_bad_angle_are_handled_and_the_bad_value_is_named() {
    let top = top_after_replacing(
        r#"<Features><Location x="0" y="0"/><Line startX="1" startY="2" endX="3" endY="2">"#,
        r#"<Features><Xform scale="2"/><Location x="0" y="0"/><Line startX="1" startY="2" endX="3" endY="2">"#,
    );
    assert_eq!(top.line_items[0].to_nm, (6_000_000, 4_000_000));
    let text = REV_C.replace(
        r#"<Features><Location x="0" y="0"/><Line startX="1" startY="2" endX="3" endY="2">"#,
        r#"<Features><Xform rotation="abc" scale="-1"/><Location x="0" y="0"/><Line startX="1" startY="2" endX="3" endY="2">"#,
    );
    let doc = read_str(&text, &options()).unwrap();
    let named = |key: &str| {
        doc.findings
            .iter()
            .any(|f| f.subject.as_deref() == Some(key))
    };
    assert!(named("Xform@rotation") && named("Xform@scale"));
}

#[test]
fn a_file_without_xform_keeps_the_coordinates_and_counts_no_placed_feature() {
    let doc = read_str(REV_C, &options()).unwrap();
    let top = doc.steps[0]
        .contents
        .iter()
        .find(|c| c.layer == "TOP")
        .unwrap();
    assert_eq!(top.placed_features, 0);
}
