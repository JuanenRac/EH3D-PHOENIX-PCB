<?xml version="1.0" encoding="utf-8"?>
<!-- EH3D-PHOENIX-PCB starter kit. SPDX-License-Identifier: GPL-3.0-or-later. Own work, not verified. -->
<eagle version="9.6.2">
<drawing>
<settings/>
<layers>
<layer number="91" name="Nets" color="1" fill="1" visible="yes" active="yes"/>
<layer number="94" name="Symbols" color="1" fill="1" visible="yes" active="yes"/>
<layer number="95" name="Names" color="1" fill="1" visible="yes" active="yes"/>
<layer number="96" name="Values" color="1" fill="1" visible="yes" active="yes"/>
</layers>
<schematic>
<libraries>
<library name="eh3d-kit">
<packages>
<package name="MODULE-STM32-TYPE">
<description>Module of the STM32 type, 2 x 20 pins, rows 15.24 mm apart (indicative sizes: measure yours). Sizes: indicative only: a generic module on 2.54 mm pin strips, not the drawing of any product. Not verified against a datasheet.</description>
<pad name="1" x="-7.62" y="24.13" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-7.62" y="21.59" drill="1" diameter="1.7"/>
<pad name="3" x="-7.62" y="19.05" drill="1" diameter="1.7"/>
<pad name="4" x="-7.62" y="16.51" drill="1" diameter="1.7"/>
<pad name="5" x="-7.62" y="13.97" drill="1" diameter="1.7"/>
<pad name="6" x="-7.62" y="11.43" drill="1" diameter="1.7"/>
<pad name="7" x="-7.62" y="8.89" drill="1" diameter="1.7"/>
<pad name="8" x="-7.62" y="6.35" drill="1" diameter="1.7"/>
<pad name="9" x="-7.62" y="3.81" drill="1" diameter="1.7"/>
<pad name="10" x="-7.62" y="1.27" drill="1" diameter="1.7"/>
<pad name="11" x="-7.62" y="-1.27" drill="1" diameter="1.7"/>
<pad name="12" x="-7.62" y="-3.81" drill="1" diameter="1.7"/>
<pad name="13" x="-7.62" y="-6.35" drill="1" diameter="1.7"/>
<pad name="14" x="-7.62" y="-8.89" drill="1" diameter="1.7"/>
<pad name="15" x="-7.62" y="-11.43" drill="1" diameter="1.7"/>
<pad name="16" x="-7.62" y="-13.97" drill="1" diameter="1.7"/>
<pad name="17" x="-7.62" y="-16.51" drill="1" diameter="1.7"/>
<pad name="18" x="-7.62" y="-19.05" drill="1" diameter="1.7"/>
<pad name="19" x="-7.62" y="-21.59" drill="1" diameter="1.7"/>
<pad name="20" x="-7.62" y="-24.13" drill="1" diameter="1.7"/>
<pad name="21" x="7.62" y="-24.13" drill="1" diameter="1.7"/>
<pad name="22" x="7.62" y="-21.59" drill="1" diameter="1.7"/>
<pad name="23" x="7.62" y="-19.05" drill="1" diameter="1.7"/>
<pad name="24" x="7.62" y="-16.51" drill="1" diameter="1.7"/>
<pad name="25" x="7.62" y="-13.97" drill="1" diameter="1.7"/>
<pad name="26" x="7.62" y="-11.43" drill="1" diameter="1.7"/>
<pad name="27" x="7.62" y="-8.89" drill="1" diameter="1.7"/>
<pad name="28" x="7.62" y="-6.35" drill="1" diameter="1.7"/>
<pad name="29" x="7.62" y="-3.81" drill="1" diameter="1.7"/>
<pad name="30" x="7.62" y="-1.27" drill="1" diameter="1.7"/>
<pad name="31" x="7.62" y="1.27" drill="1" diameter="1.7"/>
<pad name="32" x="7.62" y="3.81" drill="1" diameter="1.7"/>
<pad name="33" x="7.62" y="6.35" drill="1" diameter="1.7"/>
<pad name="34" x="7.62" y="8.89" drill="1" diameter="1.7"/>
<pad name="35" x="7.62" y="11.43" drill="1" diameter="1.7"/>
<pad name="36" x="7.62" y="13.97" drill="1" diameter="1.7"/>
<pad name="37" x="7.62" y="16.51" drill="1" diameter="1.7"/>
<pad name="38" x="7.62" y="19.05" drill="1" diameter="1.7"/>
<pad name="39" x="7.62" y="21.59" drill="1" diameter="1.7"/>
<pad name="40" x="7.62" y="24.13" drill="1" diameter="1.7"/>
<wire x1="-9.12" y1="-25.63" x2="9.12" y2="-25.63" width="0.15" layer="21"/>
<wire x1="9.12" y1="-25.63" x2="9.12" y2="25.63" width="0.15" layer="21"/>
<wire x1="9.12" y1="25.63" x2="-9.12" y2="25.63" width="0.15" layer="21"/>
<wire x1="-9.12" y1="25.63" x2="-9.12" y2="-25.63" width="0.15" layer="21"/>
<text x="0" y="25.93" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-26.93" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="HDR-1X20">
<description>Pin header, 1 row of 20, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-24.13" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-21.59" y="0" drill="1" diameter="1.7"/>
<pad name="3" x="-19.05" y="0" drill="1" diameter="1.7"/>
<pad name="4" x="-16.51" y="0" drill="1" diameter="1.7"/>
<pad name="5" x="-13.97" y="0" drill="1" diameter="1.7"/>
<pad name="6" x="-11.43" y="0" drill="1" diameter="1.7"/>
<pad name="7" x="-8.89" y="0" drill="1" diameter="1.7"/>
<pad name="8" x="-6.35" y="0" drill="1" diameter="1.7"/>
<pad name="9" x="-3.81" y="0" drill="1" diameter="1.7"/>
<pad name="10" x="-1.27" y="0" drill="1" diameter="1.7"/>
<pad name="11" x="1.27" y="0" drill="1" diameter="1.7"/>
<pad name="12" x="3.81" y="0" drill="1" diameter="1.7"/>
<pad name="13" x="6.35" y="0" drill="1" diameter="1.7"/>
<pad name="14" x="8.89" y="0" drill="1" diameter="1.7"/>
<pad name="15" x="11.43" y="0" drill="1" diameter="1.7"/>
<pad name="16" x="13.97" y="0" drill="1" diameter="1.7"/>
<pad name="17" x="16.51" y="0" drill="1" diameter="1.7"/>
<pad name="18" x="19.05" y="0" drill="1" diameter="1.7"/>
<pad name="19" x="21.59" y="0" drill="1" diameter="1.7"/>
<pad name="20" x="24.13" y="0" drill="1" diameter="1.7"/>
<wire x1="-25.4" y1="-1.27" x2="25.4" y2="-1.27" width="0.15" layer="21"/>
<wire x1="25.4" y1="-1.27" x2="25.4" y2="1.27" width="0.15" layer="21"/>
<wire x1="25.4" y1="1.27" x2="-25.4" y2="1.27" width="0.15" layer="21"/>
<wire x1="-25.4" y1="1.27" x2="-25.4" y2="-1.27" width="0.15" layer="21"/>
<text x="0" y="1.6" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.6" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="CHIP-0402">
<description>Chip resistor or capacitor 0402. Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.48" y="0" dx="0.56" dy="0.62" layer="1"/>
<smd name="2" x="0.48" y="0" dx="0.56" dy="0.62" layer="1"/>
<wire x1="-0.2" y1="0.61" x2="0.2" y2="0.61" width="0.15" layer="21"/>
<wire x1="-0.2" y1="-0.61" x2="0.2" y2="-0.61" width="0.15" layer="21"/>
<wire x1="-0.5" y1="-0.25" x2="0.5" y2="-0.25" width="0.1" layer="51"/>
<wire x1="0.5" y1="-0.25" x2="0.5" y2="0.25" width="0.1" layer="51"/>
<wire x1="0.5" y1="0.25" x2="-0.5" y2="0.25" width="0.1" layer="51"/>
<wire x1="-0.5" y1="0.25" x2="-0.5" y2="-0.25" width="0.1" layer="51"/>
<text x="0" y="0.91" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-1.91" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="CHIP-0603">
<description>Chip resistor or capacitor 0603. Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.8" y="0" dx="0.9" dy="0.95" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.9" dy="0.95" layer="1"/>
<wire x1="-0.35" y1="0.775" x2="0.35" y2="0.775" width="0.15" layer="21"/>
<wire x1="-0.35" y1="-0.775" x2="0.35" y2="-0.775" width="0.15" layer="21"/>
<wire x1="-0.8" y1="-0.4" x2="0.8" y2="-0.4" width="0.1" layer="51"/>
<wire x1="0.8" y1="-0.4" x2="0.8" y2="0.4" width="0.1" layer="51"/>
<wire x1="0.8" y1="0.4" x2="-0.8" y2="0.4" width="0.1" layer="51"/>
<wire x1="-0.8" y1="0.4" x2="-0.8" y2="-0.4" width="0.1" layer="51"/>
<text x="0" y="1.075" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.075" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="CHIP-0805">
<description>Chip resistor or capacitor 0805. Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.95" y="0" dx="1.15" dy="1.45" layer="1"/>
<smd name="2" x="0.95" y="0" dx="1.15" dy="1.45" layer="1"/>
<wire x1="-0.375" y1="1.025" x2="0.375" y2="1.025" width="0.15" layer="21"/>
<wire x1="-0.375" y1="-1.025" x2="0.375" y2="-1.025" width="0.15" layer="21"/>
<wire x1="-1" y1="-0.625" x2="1" y2="-0.625" width="0.1" layer="51"/>
<wire x1="1" y1="-0.625" x2="1" y2="0.625" width="0.1" layer="51"/>
<wire x1="1" y1="0.625" x2="-1" y2="0.625" width="0.1" layer="51"/>
<wire x1="-1" y1="0.625" x2="-1" y2="-0.625" width="0.1" layer="51"/>
<text x="0" y="1.325" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.325" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="CHIP-1206">
<description>Chip resistor or capacitor 1206. Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-1.5" y="0" dx="1.15" dy="1.75" layer="1"/>
<smd name="2" x="1.5" y="0" dx="1.15" dy="1.75" layer="1"/>
<wire x1="-0.925" y1="1.175" x2="0.925" y2="1.175" width="0.15" layer="21"/>
<wire x1="-0.925" y1="-1.175" x2="0.925" y2="-1.175" width="0.15" layer="21"/>
<wire x1="-1.6" y1="-0.8" x2="1.6" y2="-0.8" width="0.1" layer="51"/>
<wire x1="1.6" y1="-0.8" x2="1.6" y2="0.8" width="0.1" layer="51"/>
<wire x1="1.6" y1="0.8" x2="-1.6" y2="0.8" width="0.1" layer="51"/>
<wire x1="-1.6" y1="0.8" x2="-1.6" y2="-0.8" width="0.1" layer="51"/>
<text x="0" y="1.475" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.475" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="C-DISC-2.5">
<description>Disc or film capacitor, lead pitch 2.5 mm. Sizes: IEC 60384 usual lead pitches; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1.25" y="0" drill="0.7" diameter="1.4" shape="square"/>
<pad name="2" x="1.25" y="0" drill="0.7" diameter="1.4"/>
<wire x1="-2.25" y1="1.1" x2="2.25" y2="1.1" width="0.15" layer="21"/>
<wire x1="-2.25" y1="-1.1" x2="2.25" y2="-1.1" width="0.15" layer="21"/>
<text x="0" y="1.4" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.4" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="C-DISC-5.0">
<description>Disc or film capacitor, lead pitch 5.0 mm. Sizes: IEC 60384 usual lead pitches; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-2.5" y="0" drill="0.7" diameter="1.4" shape="square"/>
<pad name="2" x="2.5" y="0" drill="0.7" diameter="1.4"/>
<wire x1="-3.5" y1="1.1" x2="3.5" y2="1.1" width="0.15" layer="21"/>
<wire x1="-3.5" y1="-1.1" x2="3.5" y2="-1.1" width="0.15" layer="21"/>
<text x="0" y="1.4" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.4" size="1" layer="27">&gt;VALUE</text>
</package>
</packages>
<symbols>
<symbol name="MODULE-40">
<pin name="1" x="-10.16" y="24.13" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="21.59" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="19.05" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="16.51" length="short" direction="pas"/>
<pin name="5" x="-10.16" y="13.97" length="short" direction="pas"/>
<pin name="6" x="-10.16" y="11.43" length="short" direction="pas"/>
<pin name="7" x="-10.16" y="8.89" length="short" direction="pas"/>
<pin name="8" x="-10.16" y="6.35" length="short" direction="pas"/>
<pin name="9" x="-10.16" y="3.81" length="short" direction="pas"/>
<pin name="10" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="11" x="-10.16" y="-1.27" length="short" direction="pas"/>
<pin name="12" x="-10.16" y="-3.81" length="short" direction="pas"/>
<pin name="13" x="-10.16" y="-6.35" length="short" direction="pas"/>
<pin name="14" x="-10.16" y="-8.89" length="short" direction="pas"/>
<pin name="15" x="-10.16" y="-11.43" length="short" direction="pas"/>
<pin name="16" x="-10.16" y="-13.97" length="short" direction="pas"/>
<pin name="17" x="-10.16" y="-16.51" length="short" direction="pas"/>
<pin name="18" x="-10.16" y="-19.05" length="short" direction="pas"/>
<pin name="19" x="-10.16" y="-21.59" length="short" direction="pas"/>
<pin name="20" x="-10.16" y="-24.13" length="short" direction="pas"/>
<pin name="40" x="10.16" y="24.13" length="short" direction="pas" rot="R180"/>
<pin name="39" x="10.16" y="21.59" length="short" direction="pas" rot="R180"/>
<pin name="38" x="10.16" y="19.05" length="short" direction="pas" rot="R180"/>
<pin name="37" x="10.16" y="16.51" length="short" direction="pas" rot="R180"/>
<pin name="36" x="10.16" y="13.97" length="short" direction="pas" rot="R180"/>
<pin name="35" x="10.16" y="11.43" length="short" direction="pas" rot="R180"/>
<pin name="34" x="10.16" y="8.89" length="short" direction="pas" rot="R180"/>
<pin name="33" x="10.16" y="6.35" length="short" direction="pas" rot="R180"/>
<pin name="32" x="10.16" y="3.81" length="short" direction="pas" rot="R180"/>
<pin name="31" x="10.16" y="1.27" length="short" direction="pas" rot="R180"/>
<pin name="30" x="10.16" y="-1.27" length="short" direction="pas" rot="R180"/>
<pin name="29" x="10.16" y="-3.81" length="short" direction="pas" rot="R180"/>
<pin name="28" x="10.16" y="-6.35" length="short" direction="pas" rot="R180"/>
<pin name="27" x="10.16" y="-8.89" length="short" direction="pas" rot="R180"/>
<pin name="26" x="10.16" y="-11.43" length="short" direction="pas" rot="R180"/>
<pin name="25" x="10.16" y="-13.97" length="short" direction="pas" rot="R180"/>
<pin name="24" x="10.16" y="-16.51" length="short" direction="pas" rot="R180"/>
<pin name="23" x="10.16" y="-19.05" length="short" direction="pas" rot="R180"/>
<pin name="22" x="10.16" y="-21.59" length="short" direction="pas" rot="R180"/>
<pin name="21" x="10.16" y="-24.13" length="short" direction="pas" rot="R180"/>
<wire x1="-7.62" y1="-26.67" x2="7.62" y2="-26.67" width="0.254" layer="94"/>
<wire x1="7.62" y1="-26.67" x2="7.62" y2="26.67" width="0.254" layer="94"/>
<wire x1="7.62" y1="26.67" x2="-7.62" y2="26.67" width="0.254" layer="94"/>
<wire x1="-7.62" y1="26.67" x2="-7.62" y2="-26.67" width="0.254" layer="94"/>
<text x="-2.54" y="26.924" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-28.702" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="PINS-1X20">
<pin name="1" x="-10.16" y="24.13" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="21.59" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="19.05" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="16.51" length="short" direction="pas"/>
<pin name="5" x="-10.16" y="13.97" length="short" direction="pas"/>
<pin name="6" x="-10.16" y="11.43" length="short" direction="pas"/>
<pin name="7" x="-10.16" y="8.89" length="short" direction="pas"/>
<pin name="8" x="-10.16" y="6.35" length="short" direction="pas"/>
<pin name="9" x="-10.16" y="3.81" length="short" direction="pas"/>
<pin name="10" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="11" x="-10.16" y="-1.27" length="short" direction="pas"/>
<pin name="12" x="-10.16" y="-3.81" length="short" direction="pas"/>
<pin name="13" x="-10.16" y="-6.35" length="short" direction="pas"/>
<pin name="14" x="-10.16" y="-8.89" length="short" direction="pas"/>
<pin name="15" x="-10.16" y="-11.43" length="short" direction="pas"/>
<pin name="16" x="-10.16" y="-13.97" length="short" direction="pas"/>
<pin name="17" x="-10.16" y="-16.51" length="short" direction="pas"/>
<pin name="18" x="-10.16" y="-19.05" length="short" direction="pas"/>
<pin name="19" x="-10.16" y="-21.59" length="short" direction="pas"/>
<pin name="20" x="-10.16" y="-24.13" length="short" direction="pas"/>
<wire x1="-7.62" y1="-26.67" x2="7.62" y2="-26.67" width="0.254" layer="94"/>
<wire x1="7.62" y1="-26.67" x2="7.62" y2="26.67" width="0.254" layer="94"/>
<wire x1="7.62" y1="26.67" x2="-7.62" y2="26.67" width="0.254" layer="94"/>
<wire x1="-7.62" y1="26.67" x2="-7.62" y2="-26.67" width="0.254" layer="94"/>
<text x="-2.54" y="26.924" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-28.702" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="CAPACITOR">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="0" x2="-0.508" y2="0" width="0.152" layer="94"/>
<wire x1="-0.508" y1="-1.524" x2="-0.508" y2="1.524" width="0.406" layer="94"/>
<wire x1="0.508" y1="-1.524" x2="0.508" y2="1.524" width="0.406" layer="94"/>
<wire x1="0.508" y1="0" x2="2.54" y2="0" width="0.152" layer="94"/>
<text x="-2.54" y="2.54" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-4.318" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
</symbols>
<devicesets>
<deviceset name="MODULE-STM32-TYPE" prefix="M" uservalue="yes">
<description>Module of the STM32 type (indicative)</description>
<gates>
<gate name="G$1" symbol="MODULE-40" x="0" y="0"/>
</gates>
<devices>
<device name="" package="MODULE-STM32-TYPE">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="4" pad="4"/>
<connect gate="G$1" pin="5" pad="5"/>
<connect gate="G$1" pin="6" pad="6"/>
<connect gate="G$1" pin="7" pad="7"/>
<connect gate="G$1" pin="8" pad="8"/>
<connect gate="G$1" pin="9" pad="9"/>
<connect gate="G$1" pin="10" pad="10"/>
<connect gate="G$1" pin="11" pad="11"/>
<connect gate="G$1" pin="12" pad="12"/>
<connect gate="G$1" pin="13" pad="13"/>
<connect gate="G$1" pin="14" pad="14"/>
<connect gate="G$1" pin="15" pad="15"/>
<connect gate="G$1" pin="16" pad="16"/>
<connect gate="G$1" pin="17" pad="17"/>
<connect gate="G$1" pin="18" pad="18"/>
<connect gate="G$1" pin="19" pad="19"/>
<connect gate="G$1" pin="20" pad="20"/>
<connect gate="G$1" pin="40" pad="40"/>
<connect gate="G$1" pin="39" pad="39"/>
<connect gate="G$1" pin="38" pad="38"/>
<connect gate="G$1" pin="37" pad="37"/>
<connect gate="G$1" pin="36" pad="36"/>
<connect gate="G$1" pin="35" pad="35"/>
<connect gate="G$1" pin="34" pad="34"/>
<connect gate="G$1" pin="33" pad="33"/>
<connect gate="G$1" pin="32" pad="32"/>
<connect gate="G$1" pin="31" pad="31"/>
<connect gate="G$1" pin="30" pad="30"/>
<connect gate="G$1" pin="29" pad="29"/>
<connect gate="G$1" pin="28" pad="28"/>
<connect gate="G$1" pin="27" pad="27"/>
<connect gate="G$1" pin="26" pad="26"/>
<connect gate="G$1" pin="25" pad="25"/>
<connect gate="G$1" pin="24" pad="24"/>
<connect gate="G$1" pin="23" pad="23"/>
<connect gate="G$1" pin="22" pad="22"/>
<connect gate="G$1" pin="21" pad="21"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="HDR-1X20" prefix="J" uservalue="yes">
<description>Pin header 1 x 20</description>
<gates>
<gate name="G$1" symbol="PINS-1X20" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X20">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="4" pad="4"/>
<connect gate="G$1" pin="5" pad="5"/>
<connect gate="G$1" pin="6" pad="6"/>
<connect gate="G$1" pin="7" pad="7"/>
<connect gate="G$1" pin="8" pad="8"/>
<connect gate="G$1" pin="9" pad="9"/>
<connect gate="G$1" pin="10" pad="10"/>
<connect gate="G$1" pin="11" pad="11"/>
<connect gate="G$1" pin="12" pad="12"/>
<connect gate="G$1" pin="13" pad="13"/>
<connect gate="G$1" pin="14" pad="14"/>
<connect gate="G$1" pin="15" pad="15"/>
<connect gate="G$1" pin="16" pad="16"/>
<connect gate="G$1" pin="17" pad="17"/>
<connect gate="G$1" pin="18" pad="18"/>
<connect gate="G$1" pin="19" pad="19"/>
<connect gate="G$1" pin="20" pad="20"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="C" prefix="C" uservalue="yes">
<description>Capacitor</description>
<gates>
<gate name="G$1" symbol="CAPACITOR" x="0" y="0"/>
</gates>
<devices>
<device name="0402" package="CHIP-0402">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="0603" package="CHIP-0603">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="0805" package="CHIP-0805">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="1206" package="CHIP-1206">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="DISC-2.5" package="C-DISC-2.5">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="DISC-5.0" package="C-DISC-5.0">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
</devicesets>
</library>
</libraries>
<parts>
<part name="M1" library="eh3d-kit" deviceset="MODULE-STM32-TYPE" device="" value="STM32-TYPE"/>
<part name="J1" library="eh3d-kit" deviceset="HDR-1X20" device="" value="LEFT"/>
<part name="J2" library="eh3d-kit" deviceset="HDR-1X20" device="" value="RIGHT"/>
<part name="C1" library="eh3d-kit" deviceset="C" device="0603" value="100n"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="M1" gate="G$1" x="30" y="50.8"/>
<instance part="J1" gate="G$1" x="30" y="114.3"/>
<instance part="J2" gate="G$1" x="85.88" y="50.8"/>
<instance part="C1" gate="G$1" x="85.88" y="88.9"/>
</instances>
<nets>
<net name="V3V3" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="74.93" x2="17.3" y2="74.93" width="0.152" layer="91"/>
<label x="17.3" y="74.93" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="138.43" x2="17.3" y2="138.43" width="0.152" layer="91"/>
<label x="17.3" y="138.43" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="1"/>
<wire x1="80.8" y1="88.9" x2="78.26" y2="88.9" width="0.152" layer="91"/>
<label x="78.26" y="88.9" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="72.39" x2="17.3" y2="72.39" width="0.152" layer="91"/>
<label x="17.3" y="72.39" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="135.89" x2="17.3" y2="135.89" width="0.152" layer="91"/>
<label x="17.3" y="135.89" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="2"/>
<wire x1="90.96" y1="88.9" x2="93.5" y2="88.9" width="0.152" layer="91"/>
<label x="93.5" y="88.9" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO3" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="69.85" x2="17.3" y2="69.85" width="0.152" layer="91"/>
<label x="17.3" y="69.85" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="133.35" x2="17.3" y2="133.35" width="0.152" layer="91"/>
<label x="17.3" y="133.35" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO4" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="67.31" x2="17.3" y2="67.31" width="0.152" layer="91"/>
<label x="17.3" y="67.31" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="130.81" x2="17.3" y2="130.81" width="0.152" layer="91"/>
<label x="17.3" y="130.81" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO5" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="5"/>
<wire x1="19.84" y1="64.77" x2="17.3" y2="64.77" width="0.152" layer="91"/>
<label x="17.3" y="64.77" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="5"/>
<wire x1="19.84" y1="128.27" x2="17.3" y2="128.27" width="0.152" layer="91"/>
<label x="17.3" y="128.27" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO6" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="6"/>
<wire x1="19.84" y1="62.23" x2="17.3" y2="62.23" width="0.152" layer="91"/>
<label x="17.3" y="62.23" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="6"/>
<wire x1="19.84" y1="125.73" x2="17.3" y2="125.73" width="0.152" layer="91"/>
<label x="17.3" y="125.73" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO7" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="7"/>
<wire x1="19.84" y1="59.69" x2="17.3" y2="59.69" width="0.152" layer="91"/>
<label x="17.3" y="59.69" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="7"/>
<wire x1="19.84" y1="123.19" x2="17.3" y2="123.19" width="0.152" layer="91"/>
<label x="17.3" y="123.19" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO8" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="8"/>
<wire x1="19.84" y1="57.15" x2="17.3" y2="57.15" width="0.152" layer="91"/>
<label x="17.3" y="57.15" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="8"/>
<wire x1="19.84" y1="120.65" x2="17.3" y2="120.65" width="0.152" layer="91"/>
<label x="17.3" y="120.65" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO9" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="9"/>
<wire x1="19.84" y1="54.61" x2="17.3" y2="54.61" width="0.152" layer="91"/>
<label x="17.3" y="54.61" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="9"/>
<wire x1="19.84" y1="118.11" x2="17.3" y2="118.11" width="0.152" layer="91"/>
<label x="17.3" y="118.11" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO10" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="10"/>
<wire x1="19.84" y1="52.07" x2="17.3" y2="52.07" width="0.152" layer="91"/>
<label x="17.3" y="52.07" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="10"/>
<wire x1="19.84" y1="115.57" x2="17.3" y2="115.57" width="0.152" layer="91"/>
<label x="17.3" y="115.57" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO11" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="11"/>
<wire x1="19.84" y1="49.53" x2="17.3" y2="49.53" width="0.152" layer="91"/>
<label x="17.3" y="49.53" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="11"/>
<wire x1="19.84" y1="113.03" x2="17.3" y2="113.03" width="0.152" layer="91"/>
<label x="17.3" y="113.03" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO12" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="12"/>
<wire x1="19.84" y1="46.99" x2="17.3" y2="46.99" width="0.152" layer="91"/>
<label x="17.3" y="46.99" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="12"/>
<wire x1="19.84" y1="110.49" x2="17.3" y2="110.49" width="0.152" layer="91"/>
<label x="17.3" y="110.49" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO13" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="13"/>
<wire x1="19.84" y1="44.45" x2="17.3" y2="44.45" width="0.152" layer="91"/>
<label x="17.3" y="44.45" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="13"/>
<wire x1="19.84" y1="107.95" x2="17.3" y2="107.95" width="0.152" layer="91"/>
<label x="17.3" y="107.95" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO14" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="14"/>
<wire x1="19.84" y1="41.91" x2="17.3" y2="41.91" width="0.152" layer="91"/>
<label x="17.3" y="41.91" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="14"/>
<wire x1="19.84" y1="105.41" x2="17.3" y2="105.41" width="0.152" layer="91"/>
<label x="17.3" y="105.41" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO15" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="15"/>
<wire x1="19.84" y1="39.37" x2="17.3" y2="39.37" width="0.152" layer="91"/>
<label x="17.3" y="39.37" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="15"/>
<wire x1="19.84" y1="102.87" x2="17.3" y2="102.87" width="0.152" layer="91"/>
<label x="17.3" y="102.87" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO16" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="16"/>
<wire x1="19.84" y1="36.83" x2="17.3" y2="36.83" width="0.152" layer="91"/>
<label x="17.3" y="36.83" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="16"/>
<wire x1="19.84" y1="100.33" x2="17.3" y2="100.33" width="0.152" layer="91"/>
<label x="17.3" y="100.33" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO17" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="17"/>
<wire x1="19.84" y1="34.29" x2="17.3" y2="34.29" width="0.152" layer="91"/>
<label x="17.3" y="34.29" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="17"/>
<wire x1="19.84" y1="97.79" x2="17.3" y2="97.79" width="0.152" layer="91"/>
<label x="17.3" y="97.79" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO18" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="18"/>
<wire x1="19.84" y1="31.75" x2="17.3" y2="31.75" width="0.152" layer="91"/>
<label x="17.3" y="31.75" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="18"/>
<wire x1="19.84" y1="95.25" x2="17.3" y2="95.25" width="0.152" layer="91"/>
<label x="17.3" y="95.25" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO19" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="19"/>
<wire x1="19.84" y1="29.21" x2="17.3" y2="29.21" width="0.152" layer="91"/>
<label x="17.3" y="29.21" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="19"/>
<wire x1="19.84" y1="92.71" x2="17.3" y2="92.71" width="0.152" layer="91"/>
<label x="17.3" y="92.71" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO20" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="20"/>
<wire x1="19.84" y1="26.67" x2="17.3" y2="26.67" width="0.152" layer="91"/>
<label x="17.3" y="26.67" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="20"/>
<wire x1="19.84" y1="90.17" x2="17.3" y2="90.17" width="0.152" layer="91"/>
<label x="17.3" y="90.17" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO21" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="21"/>
<wire x1="40.16" y1="26.67" x2="42.7" y2="26.67" width="0.152" layer="91"/>
<label x="42.7" y="26.67" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="1"/>
<wire x1="75.72" y1="74.93" x2="73.18" y2="74.93" width="0.152" layer="91"/>
<label x="73.18" y="74.93" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO22" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="22"/>
<wire x1="40.16" y1="29.21" x2="42.7" y2="29.21" width="0.152" layer="91"/>
<label x="42.7" y="29.21" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="2"/>
<wire x1="75.72" y1="72.39" x2="73.18" y2="72.39" width="0.152" layer="91"/>
<label x="73.18" y="72.39" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO23" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="23"/>
<wire x1="40.16" y1="31.75" x2="42.7" y2="31.75" width="0.152" layer="91"/>
<label x="42.7" y="31.75" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="3"/>
<wire x1="75.72" y1="69.85" x2="73.18" y2="69.85" width="0.152" layer="91"/>
<label x="73.18" y="69.85" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO24" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="24"/>
<wire x1="40.16" y1="34.29" x2="42.7" y2="34.29" width="0.152" layer="91"/>
<label x="42.7" y="34.29" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="4"/>
<wire x1="75.72" y1="67.31" x2="73.18" y2="67.31" width="0.152" layer="91"/>
<label x="73.18" y="67.31" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO25" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="25"/>
<wire x1="40.16" y1="36.83" x2="42.7" y2="36.83" width="0.152" layer="91"/>
<label x="42.7" y="36.83" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="5"/>
<wire x1="75.72" y1="64.77" x2="73.18" y2="64.77" width="0.152" layer="91"/>
<label x="73.18" y="64.77" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO26" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="26"/>
<wire x1="40.16" y1="39.37" x2="42.7" y2="39.37" width="0.152" layer="91"/>
<label x="42.7" y="39.37" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="6"/>
<wire x1="75.72" y1="62.23" x2="73.18" y2="62.23" width="0.152" layer="91"/>
<label x="73.18" y="62.23" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO27" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="27"/>
<wire x1="40.16" y1="41.91" x2="42.7" y2="41.91" width="0.152" layer="91"/>
<label x="42.7" y="41.91" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="7"/>
<wire x1="75.72" y1="59.69" x2="73.18" y2="59.69" width="0.152" layer="91"/>
<label x="73.18" y="59.69" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO28" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="28"/>
<wire x1="40.16" y1="44.45" x2="42.7" y2="44.45" width="0.152" layer="91"/>
<label x="42.7" y="44.45" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="8"/>
<wire x1="75.72" y1="57.15" x2="73.18" y2="57.15" width="0.152" layer="91"/>
<label x="73.18" y="57.15" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO29" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="29"/>
<wire x1="40.16" y1="46.99" x2="42.7" y2="46.99" width="0.152" layer="91"/>
<label x="42.7" y="46.99" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="9"/>
<wire x1="75.72" y1="54.61" x2="73.18" y2="54.61" width="0.152" layer="91"/>
<label x="73.18" y="54.61" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO30" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="30"/>
<wire x1="40.16" y1="49.53" x2="42.7" y2="49.53" width="0.152" layer="91"/>
<label x="42.7" y="49.53" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="10"/>
<wire x1="75.72" y1="52.07" x2="73.18" y2="52.07" width="0.152" layer="91"/>
<label x="73.18" y="52.07" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO31" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="31"/>
<wire x1="40.16" y1="52.07" x2="42.7" y2="52.07" width="0.152" layer="91"/>
<label x="42.7" y="52.07" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="11"/>
<wire x1="75.72" y1="49.53" x2="73.18" y2="49.53" width="0.152" layer="91"/>
<label x="73.18" y="49.53" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO32" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="32"/>
<wire x1="40.16" y1="54.61" x2="42.7" y2="54.61" width="0.152" layer="91"/>
<label x="42.7" y="54.61" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="12"/>
<wire x1="75.72" y1="46.99" x2="73.18" y2="46.99" width="0.152" layer="91"/>
<label x="73.18" y="46.99" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO33" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="33"/>
<wire x1="40.16" y1="57.15" x2="42.7" y2="57.15" width="0.152" layer="91"/>
<label x="42.7" y="57.15" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="13"/>
<wire x1="75.72" y1="44.45" x2="73.18" y2="44.45" width="0.152" layer="91"/>
<label x="73.18" y="44.45" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO34" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="34"/>
<wire x1="40.16" y1="59.69" x2="42.7" y2="59.69" width="0.152" layer="91"/>
<label x="42.7" y="59.69" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="14"/>
<wire x1="75.72" y1="41.91" x2="73.18" y2="41.91" width="0.152" layer="91"/>
<label x="73.18" y="41.91" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO35" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="35"/>
<wire x1="40.16" y1="62.23" x2="42.7" y2="62.23" width="0.152" layer="91"/>
<label x="42.7" y="62.23" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="15"/>
<wire x1="75.72" y1="39.37" x2="73.18" y2="39.37" width="0.152" layer="91"/>
<label x="73.18" y="39.37" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO36" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="36"/>
<wire x1="40.16" y1="64.77" x2="42.7" y2="64.77" width="0.152" layer="91"/>
<label x="42.7" y="64.77" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="16"/>
<wire x1="75.72" y1="36.83" x2="73.18" y2="36.83" width="0.152" layer="91"/>
<label x="73.18" y="36.83" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO37" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="37"/>
<wire x1="40.16" y1="67.31" x2="42.7" y2="67.31" width="0.152" layer="91"/>
<label x="42.7" y="67.31" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="17"/>
<wire x1="75.72" y1="34.29" x2="73.18" y2="34.29" width="0.152" layer="91"/>
<label x="73.18" y="34.29" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO38" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="38"/>
<wire x1="40.16" y1="69.85" x2="42.7" y2="69.85" width="0.152" layer="91"/>
<label x="42.7" y="69.85" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="18"/>
<wire x1="75.72" y1="31.75" x2="73.18" y2="31.75" width="0.152" layer="91"/>
<label x="73.18" y="31.75" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO39" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="39"/>
<wire x1="40.16" y1="72.39" x2="42.7" y2="72.39" width="0.152" layer="91"/>
<label x="42.7" y="72.39" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="19"/>
<wire x1="75.72" y1="29.21" x2="73.18" y2="29.21" width="0.152" layer="91"/>
<label x="73.18" y="29.21" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO40" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="40"/>
<wire x1="40.16" y1="74.93" x2="42.7" y2="74.93" width="0.152" layer="91"/>
<label x="42.7" y="74.93" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="20"/>
<wire x1="75.72" y1="26.67" x2="73.18" y2="26.67" width="0.152" layer="91"/>
<label x="73.18" y="26.67" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
