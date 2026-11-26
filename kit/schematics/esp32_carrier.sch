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
<package name="MODULE-ESP32-TYPE">
<description>Module of the ESP32 type, 2 x 15 pins, rows 25.4 mm apart (indicative sizes: measure yours). Sizes: indicative only: a generic module on 2.54 mm pin strips, not the drawing of any product. Not verified against a datasheet.</description>
<pad name="1" x="-12.7" y="17.78" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-12.7" y="15.24" drill="1" diameter="1.7"/>
<pad name="3" x="-12.7" y="12.7" drill="1" diameter="1.7"/>
<pad name="4" x="-12.7" y="10.16" drill="1" diameter="1.7"/>
<pad name="5" x="-12.7" y="7.62" drill="1" diameter="1.7"/>
<pad name="6" x="-12.7" y="5.08" drill="1" diameter="1.7"/>
<pad name="7" x="-12.7" y="2.54" drill="1" diameter="1.7"/>
<pad name="8" x="-12.7" y="0" drill="1" diameter="1.7"/>
<pad name="9" x="-12.7" y="-2.54" drill="1" diameter="1.7"/>
<pad name="10" x="-12.7" y="-5.08" drill="1" diameter="1.7"/>
<pad name="11" x="-12.7" y="-7.62" drill="1" diameter="1.7"/>
<pad name="12" x="-12.7" y="-10.16" drill="1" diameter="1.7"/>
<pad name="13" x="-12.7" y="-12.7" drill="1" diameter="1.7"/>
<pad name="14" x="-12.7" y="-15.24" drill="1" diameter="1.7"/>
<pad name="15" x="-12.7" y="-17.78" drill="1" diameter="1.7"/>
<pad name="16" x="12.7" y="-17.78" drill="1" diameter="1.7"/>
<pad name="17" x="12.7" y="-15.24" drill="1" diameter="1.7"/>
<pad name="18" x="12.7" y="-12.7" drill="1" diameter="1.7"/>
<pad name="19" x="12.7" y="-10.16" drill="1" diameter="1.7"/>
<pad name="20" x="12.7" y="-7.62" drill="1" diameter="1.7"/>
<pad name="21" x="12.7" y="-5.08" drill="1" diameter="1.7"/>
<pad name="22" x="12.7" y="-2.54" drill="1" diameter="1.7"/>
<pad name="23" x="12.7" y="0" drill="1" diameter="1.7"/>
<pad name="24" x="12.7" y="2.54" drill="1" diameter="1.7"/>
<pad name="25" x="12.7" y="5.08" drill="1" diameter="1.7"/>
<pad name="26" x="12.7" y="7.62" drill="1" diameter="1.7"/>
<pad name="27" x="12.7" y="10.16" drill="1" diameter="1.7"/>
<pad name="28" x="12.7" y="12.7" drill="1" diameter="1.7"/>
<pad name="29" x="12.7" y="15.24" drill="1" diameter="1.7"/>
<pad name="30" x="12.7" y="17.78" drill="1" diameter="1.7"/>
<wire x1="-14.2" y1="-19.28" x2="14.2" y2="-19.28" width="0.15" layer="21"/>
<wire x1="14.2" y1="-19.28" x2="14.2" y2="19.28" width="0.15" layer="21"/>
<wire x1="14.2" y1="19.28" x2="-14.2" y2="19.28" width="0.15" layer="21"/>
<wire x1="-14.2" y1="19.28" x2="-14.2" y2="-19.28" width="0.15" layer="21"/>
<text x="0" y="19.58" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-20.58" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="HDR-1X15">
<description>Pin header, 1 row of 15, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-17.78" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-15.24" y="0" drill="1" diameter="1.7"/>
<pad name="3" x="-12.7" y="0" drill="1" diameter="1.7"/>
<pad name="4" x="-10.16" y="0" drill="1" diameter="1.7"/>
<pad name="5" x="-7.62" y="0" drill="1" diameter="1.7"/>
<pad name="6" x="-5.08" y="0" drill="1" diameter="1.7"/>
<pad name="7" x="-2.54" y="0" drill="1" diameter="1.7"/>
<pad name="8" x="0" y="0" drill="1" diameter="1.7"/>
<pad name="9" x="2.54" y="0" drill="1" diameter="1.7"/>
<pad name="10" x="5.08" y="0" drill="1" diameter="1.7"/>
<pad name="11" x="7.62" y="0" drill="1" diameter="1.7"/>
<pad name="12" x="10.16" y="0" drill="1" diameter="1.7"/>
<pad name="13" x="12.7" y="0" drill="1" diameter="1.7"/>
<pad name="14" x="15.24" y="0" drill="1" diameter="1.7"/>
<pad name="15" x="17.78" y="0" drill="1" diameter="1.7"/>
<wire x1="-19.05" y1="-1.27" x2="19.05" y2="-1.27" width="0.15" layer="21"/>
<wire x1="19.05" y1="-1.27" x2="19.05" y2="1.27" width="0.15" layer="21"/>
<wire x1="19.05" y1="1.27" x2="-19.05" y2="1.27" width="0.15" layer="21"/>
<wire x1="-19.05" y1="1.27" x2="-19.05" y2="-1.27" width="0.15" layer="21"/>
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
<symbol name="MODULE-30">
<pin name="1" x="-10.16" y="17.78" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="15.24" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="12.7" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="10.16" length="short" direction="pas"/>
<pin name="5" x="-10.16" y="7.62" length="short" direction="pas"/>
<pin name="6" x="-10.16" y="5.08" length="short" direction="pas"/>
<pin name="7" x="-10.16" y="2.54" length="short" direction="pas"/>
<pin name="8" x="-10.16" y="0" length="short" direction="pas"/>
<pin name="9" x="-10.16" y="-2.54" length="short" direction="pas"/>
<pin name="10" x="-10.16" y="-5.08" length="short" direction="pas"/>
<pin name="11" x="-10.16" y="-7.62" length="short" direction="pas"/>
<pin name="12" x="-10.16" y="-10.16" length="short" direction="pas"/>
<pin name="13" x="-10.16" y="-12.7" length="short" direction="pas"/>
<pin name="14" x="-10.16" y="-15.24" length="short" direction="pas"/>
<pin name="15" x="-10.16" y="-17.78" length="short" direction="pas"/>
<pin name="30" x="10.16" y="17.78" length="short" direction="pas" rot="R180"/>
<pin name="29" x="10.16" y="15.24" length="short" direction="pas" rot="R180"/>
<pin name="28" x="10.16" y="12.7" length="short" direction="pas" rot="R180"/>
<pin name="27" x="10.16" y="10.16" length="short" direction="pas" rot="R180"/>
<pin name="26" x="10.16" y="7.62" length="short" direction="pas" rot="R180"/>
<pin name="25" x="10.16" y="5.08" length="short" direction="pas" rot="R180"/>
<pin name="24" x="10.16" y="2.54" length="short" direction="pas" rot="R180"/>
<pin name="23" x="10.16" y="0" length="short" direction="pas" rot="R180"/>
<pin name="22" x="10.16" y="-2.54" length="short" direction="pas" rot="R180"/>
<pin name="21" x="10.16" y="-5.08" length="short" direction="pas" rot="R180"/>
<pin name="20" x="10.16" y="-7.62" length="short" direction="pas" rot="R180"/>
<pin name="19" x="10.16" y="-10.16" length="short" direction="pas" rot="R180"/>
<pin name="18" x="10.16" y="-12.7" length="short" direction="pas" rot="R180"/>
<pin name="17" x="10.16" y="-15.24" length="short" direction="pas" rot="R180"/>
<pin name="16" x="10.16" y="-17.78" length="short" direction="pas" rot="R180"/>
<wire x1="-7.62" y1="-20.32" x2="7.62" y2="-20.32" width="0.254" layer="94"/>
<wire x1="7.62" y1="-20.32" x2="7.62" y2="20.32" width="0.254" layer="94"/>
<wire x1="7.62" y1="20.32" x2="-7.62" y2="20.32" width="0.254" layer="94"/>
<wire x1="-7.62" y1="20.32" x2="-7.62" y2="-20.32" width="0.254" layer="94"/>
<text x="-2.54" y="20.574" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-22.352" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="PINS-1X15">
<pin name="1" x="-10.16" y="17.78" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="15.24" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="12.7" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="10.16" length="short" direction="pas"/>
<pin name="5" x="-10.16" y="7.62" length="short" direction="pas"/>
<pin name="6" x="-10.16" y="5.08" length="short" direction="pas"/>
<pin name="7" x="-10.16" y="2.54" length="short" direction="pas"/>
<pin name="8" x="-10.16" y="0" length="short" direction="pas"/>
<pin name="9" x="-10.16" y="-2.54" length="short" direction="pas"/>
<pin name="10" x="-10.16" y="-5.08" length="short" direction="pas"/>
<pin name="11" x="-10.16" y="-7.62" length="short" direction="pas"/>
<pin name="12" x="-10.16" y="-10.16" length="short" direction="pas"/>
<pin name="13" x="-10.16" y="-12.7" length="short" direction="pas"/>
<pin name="14" x="-10.16" y="-15.24" length="short" direction="pas"/>
<pin name="15" x="-10.16" y="-17.78" length="short" direction="pas"/>
<wire x1="-7.62" y1="-20.32" x2="7.62" y2="-20.32" width="0.254" layer="94"/>
<wire x1="7.62" y1="-20.32" x2="7.62" y2="20.32" width="0.254" layer="94"/>
<wire x1="7.62" y1="20.32" x2="-7.62" y2="20.32" width="0.254" layer="94"/>
<wire x1="-7.62" y1="20.32" x2="-7.62" y2="-20.32" width="0.254" layer="94"/>
<text x="-2.54" y="20.574" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-22.352" size="1.778" layer="96">&gt;VALUE</text>
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
<deviceset name="MODULE-ESP32-TYPE" prefix="M" uservalue="yes">
<description>Module of the ESP32 type (indicative)</description>
<gates>
<gate name="G$1" symbol="MODULE-30" x="0" y="0"/>
</gates>
<devices>
<device name="" package="MODULE-ESP32-TYPE">
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
<connect gate="G$1" pin="20" pad="20"/>
<connect gate="G$1" pin="19" pad="19"/>
<connect gate="G$1" pin="18" pad="18"/>
<connect gate="G$1" pin="17" pad="17"/>
<connect gate="G$1" pin="16" pad="16"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="HDR-1X15" prefix="J" uservalue="yes">
<description>Pin header 1 x 15</description>
<gates>
<gate name="G$1" symbol="PINS-1X15" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X15">
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
<part name="M1" library="eh3d-kit" deviceset="MODULE-ESP32-TYPE" device="" value="ESP32-TYPE"/>
<part name="J1" library="eh3d-kit" deviceset="HDR-1X15" device="" value="LEFT"/>
<part name="J2" library="eh3d-kit" deviceset="HDR-1X15" device="" value="RIGHT"/>
<part name="C1" library="eh3d-kit" deviceset="C" device="0805" value="10u"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="M1" gate="G$1" x="30" y="43.18"/>
<instance part="J1" gate="G$1" x="30" y="93.98"/>
<instance part="J2" gate="G$1" x="30" y="144.78"/>
<instance part="C1" gate="G$1" x="30" y="177.8"/>
</instances>
<nets>
<net name="V3V3" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="60.96" x2="17.3" y2="60.96" width="0.152" layer="91"/>
<label x="17.3" y="60.96" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="111.76" x2="17.3" y2="111.76" width="0.152" layer="91"/>
<label x="17.3" y="111.76" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="177.8" x2="22.38" y2="177.8" width="0.152" layer="91"/>
<label x="22.38" y="177.8" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="58.42" x2="17.3" y2="58.42" width="0.152" layer="91"/>
<label x="17.3" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="109.22" x2="17.3" y2="109.22" width="0.152" layer="91"/>
<label x="17.3" y="109.22" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="177.8" x2="37.62" y2="177.8" width="0.152" layer="91"/>
<label x="37.62" y="177.8" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO3" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="55.88" x2="17.3" y2="55.88" width="0.152" layer="91"/>
<label x="17.3" y="55.88" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="106.68" x2="17.3" y2="106.68" width="0.152" layer="91"/>
<label x="17.3" y="106.68" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO4" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="53.34" x2="17.3" y2="53.34" width="0.152" layer="91"/>
<label x="17.3" y="53.34" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="104.14" x2="17.3" y2="104.14" width="0.152" layer="91"/>
<label x="17.3" y="104.14" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO5" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="5"/>
<wire x1="19.84" y1="50.8" x2="17.3" y2="50.8" width="0.152" layer="91"/>
<label x="17.3" y="50.8" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="5"/>
<wire x1="19.84" y1="101.6" x2="17.3" y2="101.6" width="0.152" layer="91"/>
<label x="17.3" y="101.6" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO6" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="6"/>
<wire x1="19.84" y1="48.26" x2="17.3" y2="48.26" width="0.152" layer="91"/>
<label x="17.3" y="48.26" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="6"/>
<wire x1="19.84" y1="99.06" x2="17.3" y2="99.06" width="0.152" layer="91"/>
<label x="17.3" y="99.06" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO7" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="7"/>
<wire x1="19.84" y1="45.72" x2="17.3" y2="45.72" width="0.152" layer="91"/>
<label x="17.3" y="45.72" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="7"/>
<wire x1="19.84" y1="96.52" x2="17.3" y2="96.52" width="0.152" layer="91"/>
<label x="17.3" y="96.52" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO8" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="8"/>
<wire x1="19.84" y1="43.18" x2="17.3" y2="43.18" width="0.152" layer="91"/>
<label x="17.3" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="8"/>
<wire x1="19.84" y1="93.98" x2="17.3" y2="93.98" width="0.152" layer="91"/>
<label x="17.3" y="93.98" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO9" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="9"/>
<wire x1="19.84" y1="40.64" x2="17.3" y2="40.64" width="0.152" layer="91"/>
<label x="17.3" y="40.64" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="9"/>
<wire x1="19.84" y1="91.44" x2="17.3" y2="91.44" width="0.152" layer="91"/>
<label x="17.3" y="91.44" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO10" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="10"/>
<wire x1="19.84" y1="38.1" x2="17.3" y2="38.1" width="0.152" layer="91"/>
<label x="17.3" y="38.1" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="10"/>
<wire x1="19.84" y1="88.9" x2="17.3" y2="88.9" width="0.152" layer="91"/>
<label x="17.3" y="88.9" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO11" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="11"/>
<wire x1="19.84" y1="35.56" x2="17.3" y2="35.56" width="0.152" layer="91"/>
<label x="17.3" y="35.56" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="11"/>
<wire x1="19.84" y1="86.36" x2="17.3" y2="86.36" width="0.152" layer="91"/>
<label x="17.3" y="86.36" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO12" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="12"/>
<wire x1="19.84" y1="33.02" x2="17.3" y2="33.02" width="0.152" layer="91"/>
<label x="17.3" y="33.02" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="12"/>
<wire x1="19.84" y1="83.82" x2="17.3" y2="83.82" width="0.152" layer="91"/>
<label x="17.3" y="83.82" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO13" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="13"/>
<wire x1="19.84" y1="30.48" x2="17.3" y2="30.48" width="0.152" layer="91"/>
<label x="17.3" y="30.48" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="13"/>
<wire x1="19.84" y1="81.28" x2="17.3" y2="81.28" width="0.152" layer="91"/>
<label x="17.3" y="81.28" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO14" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="14"/>
<wire x1="19.84" y1="27.94" x2="17.3" y2="27.94" width="0.152" layer="91"/>
<label x="17.3" y="27.94" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="14"/>
<wire x1="19.84" y1="78.74" x2="17.3" y2="78.74" width="0.152" layer="91"/>
<label x="17.3" y="78.74" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO15" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="15"/>
<wire x1="19.84" y1="25.4" x2="17.3" y2="25.4" width="0.152" layer="91"/>
<label x="17.3" y="25.4" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="15"/>
<wire x1="19.84" y1="76.2" x2="17.3" y2="76.2" width="0.152" layer="91"/>
<label x="17.3" y="76.2" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO16" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="16"/>
<wire x1="40.16" y1="25.4" x2="42.7" y2="25.4" width="0.152" layer="91"/>
<label x="42.7" y="25.4" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="1"/>
<wire x1="19.84" y1="162.56" x2="17.3" y2="162.56" width="0.152" layer="91"/>
<label x="17.3" y="162.56" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO17" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="17"/>
<wire x1="40.16" y1="27.94" x2="42.7" y2="27.94" width="0.152" layer="91"/>
<label x="42.7" y="27.94" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="2"/>
<wire x1="19.84" y1="160.02" x2="17.3" y2="160.02" width="0.152" layer="91"/>
<label x="17.3" y="160.02" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO18" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="18"/>
<wire x1="40.16" y1="30.48" x2="42.7" y2="30.48" width="0.152" layer="91"/>
<label x="42.7" y="30.48" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="3"/>
<wire x1="19.84" y1="157.48" x2="17.3" y2="157.48" width="0.152" layer="91"/>
<label x="17.3" y="157.48" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO19" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="19"/>
<wire x1="40.16" y1="33.02" x2="42.7" y2="33.02" width="0.152" layer="91"/>
<label x="42.7" y="33.02" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="4"/>
<wire x1="19.84" y1="154.94" x2="17.3" y2="154.94" width="0.152" layer="91"/>
<label x="17.3" y="154.94" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO20" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="20"/>
<wire x1="40.16" y1="35.56" x2="42.7" y2="35.56" width="0.152" layer="91"/>
<label x="42.7" y="35.56" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="5"/>
<wire x1="19.84" y1="152.4" x2="17.3" y2="152.4" width="0.152" layer="91"/>
<label x="17.3" y="152.4" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO21" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="21"/>
<wire x1="40.16" y1="38.1" x2="42.7" y2="38.1" width="0.152" layer="91"/>
<label x="42.7" y="38.1" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="6"/>
<wire x1="19.84" y1="149.86" x2="17.3" y2="149.86" width="0.152" layer="91"/>
<label x="17.3" y="149.86" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO22" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="22"/>
<wire x1="40.16" y1="40.64" x2="42.7" y2="40.64" width="0.152" layer="91"/>
<label x="42.7" y="40.64" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="7"/>
<wire x1="19.84" y1="147.32" x2="17.3" y2="147.32" width="0.152" layer="91"/>
<label x="17.3" y="147.32" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO23" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="23"/>
<wire x1="40.16" y1="43.18" x2="42.7" y2="43.18" width="0.152" layer="91"/>
<label x="42.7" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="8"/>
<wire x1="19.84" y1="144.78" x2="17.3" y2="144.78" width="0.152" layer="91"/>
<label x="17.3" y="144.78" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO24" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="24"/>
<wire x1="40.16" y1="45.72" x2="42.7" y2="45.72" width="0.152" layer="91"/>
<label x="42.7" y="45.72" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="9"/>
<wire x1="19.84" y1="142.24" x2="17.3" y2="142.24" width="0.152" layer="91"/>
<label x="17.3" y="142.24" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO25" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="25"/>
<wire x1="40.16" y1="48.26" x2="42.7" y2="48.26" width="0.152" layer="91"/>
<label x="42.7" y="48.26" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="10"/>
<wire x1="19.84" y1="139.7" x2="17.3" y2="139.7" width="0.152" layer="91"/>
<label x="17.3" y="139.7" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO26" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="26"/>
<wire x1="40.16" y1="50.8" x2="42.7" y2="50.8" width="0.152" layer="91"/>
<label x="42.7" y="50.8" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="11"/>
<wire x1="19.84" y1="137.16" x2="17.3" y2="137.16" width="0.152" layer="91"/>
<label x="17.3" y="137.16" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO27" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="27"/>
<wire x1="40.16" y1="53.34" x2="42.7" y2="53.34" width="0.152" layer="91"/>
<label x="42.7" y="53.34" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="12"/>
<wire x1="19.84" y1="134.62" x2="17.3" y2="134.62" width="0.152" layer="91"/>
<label x="17.3" y="134.62" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO28" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="28"/>
<wire x1="40.16" y1="55.88" x2="42.7" y2="55.88" width="0.152" layer="91"/>
<label x="42.7" y="55.88" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="13"/>
<wire x1="19.84" y1="132.08" x2="17.3" y2="132.08" width="0.152" layer="91"/>
<label x="17.3" y="132.08" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO29" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="29"/>
<wire x1="40.16" y1="58.42" x2="42.7" y2="58.42" width="0.152" layer="91"/>
<label x="42.7" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="14"/>
<wire x1="19.84" y1="129.54" x2="17.3" y2="129.54" width="0.152" layer="91"/>
<label x="17.3" y="129.54" size="1.778" layer="95"/>
</segment>
</net>
<net name="IO30" class="0">
<segment>
<pinref part="M1" gate="G$1" pin="30"/>
<wire x1="40.16" y1="60.96" x2="42.7" y2="60.96" width="0.152" layer="91"/>
<label x="42.7" y="60.96" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="15"/>
<wire x1="19.84" y1="127" x2="17.3" y2="127" width="0.152" layer="91"/>
<label x="17.3" y="127" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
