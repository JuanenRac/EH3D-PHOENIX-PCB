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
<package name="HDR-1X03">
<description>Pin header, 1 row of 3, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-2.54" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="0" y="0" drill="1" diameter="1.7"/>
<pad name="3" x="2.54" y="0" drill="1" diameter="1.7"/>
<wire x1="-3.81" y1="-1.27" x2="3.81" y2="-1.27" width="0.15" layer="21"/>
<wire x1="3.81" y1="-1.27" x2="3.81" y2="1.27" width="0.15" layer="21"/>
<wire x1="3.81" y1="1.27" x2="-3.81" y2="1.27" width="0.15" layer="21"/>
<wire x1="-3.81" y1="1.27" x2="-3.81" y2="-1.27" width="0.15" layer="21"/>
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
<package name="R-AXIAL-7.62">
<description>Axial resistor, lead pitch 7.62 mm. Sizes: 0.1 inch lead grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-3.81" y="0" drill="0.8" diameter="1.6" shape="square"/>
<pad name="2" x="3.81" y="0" drill="0.8" diameter="1.6"/>
<wire x1="-2.51" y1="-1.2" x2="2.51" y2="-1.2" width="0.15" layer="21"/>
<wire x1="2.51" y1="-1.2" x2="2.51" y2="1.2" width="0.15" layer="21"/>
<wire x1="2.51" y1="1.2" x2="-2.51" y2="1.2" width="0.15" layer="21"/>
<wire x1="-2.51" y1="1.2" x2="-2.51" y2="-1.2" width="0.15" layer="21"/>
<text x="0" y="1.5" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.5" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="R-AXIAL-10.16">
<description>Axial resistor, lead pitch 10.16 mm. Sizes: 0.1 inch lead grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-5.08" y="0" drill="0.8" diameter="1.6" shape="square"/>
<pad name="2" x="5.08" y="0" drill="0.8" diameter="1.6"/>
<wire x1="-3.78" y1="-1.2" x2="3.78" y2="-1.2" width="0.15" layer="21"/>
<wire x1="3.78" y1="-1.2" x2="3.78" y2="1.2" width="0.15" layer="21"/>
<wire x1="3.78" y1="1.2" x2="-3.78" y2="1.2" width="0.15" layer="21"/>
<wire x1="-3.78" y1="1.2" x2="-3.78" y2="-1.2" width="0.15" layer="21"/>
<text x="0" y="1.5" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.5" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="SOT-23">
<description>Small outline transistor, 3 pins. Sizes: JEDEC TO-236 nominal sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.95" y="-1" dx="0.7" dy="0.9" layer="1"/>
<smd name="2" x="0.95" y="-1" dx="0.7" dy="0.9" layer="1"/>
<smd name="3" x="0" y="1" dx="0.7" dy="0.9" layer="1"/>
<wire x1="-1.6" y1="-0.2" x2="-1.6" y2="0.9" width="0.15" layer="21"/>
<wire x1="1.6" y1="-0.2" x2="1.6" y2="0.9" width="0.15" layer="21"/>
<wire x1="-1.45" y1="-0.65" x2="1.45" y2="-0.65" width="0.1" layer="51"/>
<wire x1="1.45" y1="-0.65" x2="1.45" y2="0.65" width="0.1" layer="51"/>
<wire x1="1.45" y1="0.65" x2="-1.45" y2="0.65" width="0.1" layer="51"/>
<wire x1="-1.45" y1="0.65" x2="-1.45" y2="-0.65" width="0.1" layer="51"/>
<text x="0" y="1.9" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.9" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="SOD-123">
<description>Small outline diode SOD-123, pad 1 is the cathode (marked). Sizes: JEDEC SOD-123 nominal sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-1.65" y="0" dx="1" dy="1.1" layer="1"/>
<smd name="2" x="1.65" y="0" dx="1" dy="1.1" layer="1"/>
<wire x1="-1.15" y1="0.85" x2="1.15" y2="0.85" width="0.15" layer="21"/>
<wire x1="-1.15" y1="-0.85" x2="1.15" y2="-0.85" width="0.15" layer="21"/>
<wire x1="-2.5" y1="-0.85" x2="-2.5" y2="0.85" width="0.15" layer="21"/>
<wire x1="-1.35" y1="-0.55" x2="1.35" y2="-0.55" width="0.1" layer="51"/>
<wire x1="1.35" y1="-0.55" x2="1.35" y2="0.55" width="0.1" layer="51"/>
<wire x1="1.35" y1="0.55" x2="-1.35" y2="0.55" width="0.1" layer="51"/>
<wire x1="-1.35" y1="0.55" x2="-1.35" y2="-0.55" width="0.1" layer="51"/>
<text x="0" y="1.15" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.15" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="SOD-323">
<description>Small outline diode SOD-323, pad 1 is the cathode (marked). Sizes: JEDEC SOD-323 nominal sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-1.1" y="0" dx="0.75" dy="0.6" layer="1"/>
<smd name="2" x="1.1" y="0" dx="0.75" dy="0.6" layer="1"/>
<wire x1="-0.725" y1="0.6" x2="0.725" y2="0.6" width="0.15" layer="21"/>
<wire x1="-0.725" y1="-0.6" x2="0.725" y2="-0.6" width="0.15" layer="21"/>
<wire x1="-1.825" y1="-0.6" x2="-1.825" y2="0.6" width="0.15" layer="21"/>
<wire x1="-0.85" y1="-0.3" x2="0.85" y2="-0.3" width="0.1" layer="51"/>
<wire x1="0.85" y1="-0.3" x2="0.85" y2="0.3" width="0.1" layer="51"/>
<wire x1="0.85" y1="0.3" x2="-0.85" y2="0.3" width="0.1" layer="51"/>
<wire x1="-0.85" y1="0.3" x2="-0.85" y2="-0.3" width="0.1" layer="51"/>
<text x="0" y="0.9" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-1.9" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="TERM-2P-5.0">
<description>Screw terminal, 2 poles, pitch 5.0 mm (generic). Sizes: usual 5.0 mm terminal pitch; generic outline, pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-2.5" y="0" drill="1.3" diameter="2.4" shape="square"/>
<pad name="2" x="2.5" y="0" drill="1.3" diameter="2.4"/>
<wire x1="-5" y1="-3.8" x2="5" y2="-3.8" width="0.15" layer="21"/>
<wire x1="5" y1="-3.8" x2="5" y2="3.8" width="0.15" layer="21"/>
<wire x1="5" y1="3.8" x2="-5" y2="3.8" width="0.15" layer="21"/>
<wire x1="-5" y1="3.8" x2="-5" y2="-3.8" width="0.15" layer="21"/>
<text x="0" y="4.1" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-5.1" size="1" layer="27">&gt;VALUE</text>
</package>
</packages>
<symbols>
<symbol name="PINS-1X03">
<pin name="1" x="-10.16" y="2.54" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="0" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="-2.54" length="short" direction="pas"/>
<wire x1="-7.62" y1="-5.08" x2="7.62" y2="-5.08" width="0.254" layer="94"/>
<wire x1="7.62" y1="-5.08" x2="7.62" y2="5.08" width="0.254" layer="94"/>
<wire x1="7.62" y1="5.08" x2="-7.62" y2="5.08" width="0.254" layer="94"/>
<wire x1="-7.62" y1="5.08" x2="-7.62" y2="-5.08" width="0.254" layer="94"/>
<text x="-2.54" y="5.334" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-7.112" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="RESISTOR">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="-1.016" x2="2.54" y2="-1.016" width="0.254" layer="94"/>
<wire x1="2.54" y1="-1.016" x2="2.54" y2="1.016" width="0.254" layer="94"/>
<wire x1="2.54" y1="1.016" x2="-2.54" y2="1.016" width="0.254" layer="94"/>
<wire x1="-2.54" y1="1.016" x2="-2.54" y2="-1.016" width="0.254" layer="94"/>
<text x="-2.54" y="2.54" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-4.318" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="TRANSISTOR-NPN">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="-2.54" length="short" direction="pas" rot="R180"/>
<pin name="3" x="5.08" y="2.54" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="0" x2="-1.27" y2="0" width="0.152" layer="94"/>
<wire x1="-1.27" y1="-1.778" x2="-1.27" y2="1.778" width="0.406" layer="94"/>
<wire x1="-1.27" y1="0.762" x2="2.54" y2="2.54" width="0.152" layer="94"/>
<wire x1="-1.27" y1="-0.762" x2="2.54" y2="-2.54" width="0.152" layer="94"/>
<wire x1="1.524" y1="-2.54" x2="2.54" y2="-2.54" width="0.152" layer="94"/>
<wire x1="1.778" y1="-1.524" x2="2.54" y2="-2.54" width="0.152" layer="94"/>
<text x="-2.54" y="3.81" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-5.588" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="DIODE">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="0" x2="2.54" y2="0" width="0.152" layer="94"/>
<wire x1="-1.27" y1="-1.27" x2="-1.27" y2="1.27" width="0.254" layer="94"/>
<wire x1="1.27" y1="-1.27" x2="1.27" y2="1.27" width="0.254" layer="94"/>
<wire x1="1.27" y1="1.27" x2="-1.27" y2="0" width="0.254" layer="94"/>
<wire x1="-1.27" y1="0" x2="1.27" y2="-1.27" width="0.254" layer="94"/>
<text x="-2.54" y="2.54" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-4.318" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="TERMINAL-2">
<pin name="1" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="-1.27" length="short" direction="pas"/>
<wire x1="-7.62" y1="-3.81" x2="7.62" y2="-3.81" width="0.254" layer="94"/>
<wire x1="7.62" y1="-3.81" x2="7.62" y2="3.81" width="0.254" layer="94"/>
<wire x1="7.62" y1="3.81" x2="-7.62" y2="3.81" width="0.254" layer="94"/>
<wire x1="-7.62" y1="3.81" x2="-7.62" y2="-3.81" width="0.254" layer="94"/>
<text x="-2.54" y="4.064" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-5.842" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
</symbols>
<devicesets>
<deviceset name="HDR-1X03" prefix="J" uservalue="yes">
<description>Pin header 1 x 3</description>
<gates>
<gate name="G$1" symbol="PINS-1X03" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X03">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="R" prefix="R" uservalue="yes">
<description>Resistor</description>
<gates>
<gate name="G$1" symbol="RESISTOR" x="0" y="0"/>
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
<device name="AXIAL-7.62" package="R-AXIAL-7.62">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="AXIAL-10.16" package="R-AXIAL-10.16">
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
<deviceset name="NPN" prefix="Q" uservalue="yes">
<description>NPN transistor, 1 base, 2 emitter, 3 collector (check your part)</description>
<gates>
<gate name="G$1" symbol="TRANSISTOR-NPN" x="0" y="0"/>
</gates>
<devices>
<device name="SOT-23" package="SOT-23">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="DIODE" prefix="D" uservalue="yes">
<description>Diode, pin 1 cathode, pin 2 anode</description>
<gates>
<gate name="G$1" symbol="DIODE" x="0" y="0"/>
</gates>
<devices>
<device name="SOD-123" package="SOD-123">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="SOD-323" package="SOD-323">
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
<deviceset name="TERMINAL-2P" prefix="X" uservalue="yes">
<description>Screw terminal, 2 poles</description>
<gates>
<gate name="G$1" symbol="TERMINAL-2" x="0" y="0"/>
</gates>
<devices>
<device name="" package="TERM-2P-5.0">
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
<part name="J1" library="eh3d-kit" deviceset="HDR-1X03" device="" value="CTRL"/>
<part name="R1" library="eh3d-kit" deviceset="R" device="0805" value="1k"/>
<part name="R2" library="eh3d-kit" deviceset="R" device="0805" value="100k"/>
<part name="Q1" library="eh3d-kit" deviceset="NPN" device="SOT-23" value="NPN"/>
<part name="D1" library="eh3d-kit" deviceset="DIODE" device="SOD-123" value="CLAMP"/>
<part name="X1" library="eh3d-kit" deviceset="TERMINAL-2P" device="" value="LOAD"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="J1" gate="G$1" x="30" y="27.94"/>
<instance part="R1" gate="G$1" x="30" y="45.72"/>
<instance part="R2" gate="G$1" x="30" y="60.96"/>
<instance part="Q1" gate="G$1" x="30" y="78.74"/>
<instance part="D1" gate="G$1" x="30" y="96.52"/>
<instance part="X1" gate="G$1" x="30" y="114.3"/>
</instances>
<nets>
<net name="CTRL" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="27.94" x2="17.3" y2="27.94" width="0.152" layer="91"/>
<label x="17.3" y="27.94" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="45.72" x2="22.38" y2="45.72" width="0.152" layer="91"/>
<label x="22.38" y="45.72" size="1.778" layer="95"/>
</segment>
</net>
<net name="BASE" class="0">
<segment>
<pinref part="R1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="45.72" x2="37.62" y2="45.72" width="0.152" layer="91"/>
<label x="37.62" y="45.72" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="Q1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="78.74" x2="22.38" y2="78.74" width="0.152" layer="91"/>
<label x="22.38" y="78.74" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="60.96" x2="22.38" y2="60.96" width="0.152" layer="91"/>
<label x="22.38" y="60.96" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="R2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="60.96" x2="37.62" y2="60.96" width="0.152" layer="91"/>
<label x="37.62" y="60.96" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="Q1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="76.2" x2="37.62" y2="76.2" width="0.152" layer="91"/>
<label x="37.62" y="76.2" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="25.4" x2="17.3" y2="25.4" width="0.152" layer="91"/>
<label x="17.3" y="25.4" size="1.778" layer="95"/>
</segment>
</net>
<net name="COIL" class="0">
<segment>
<pinref part="Q1" gate="G$1" pin="3"/>
<wire x1="35.08" y1="81.28" x2="37.62" y2="81.28" width="0.152" layer="91"/>
<label x="37.62" y="81.28" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="96.52" x2="37.62" y2="96.52" width="0.152" layer="91"/>
<label x="37.62" y="96.52" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="X1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="115.57" x2="17.3" y2="115.57" width="0.152" layer="91"/>
<label x="17.3" y="115.57" size="1.778" layer="95"/>
</segment>
</net>
<net name="VCC" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="30.48" x2="17.3" y2="30.48" width="0.152" layer="91"/>
<label x="17.3" y="30.48" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="96.52" x2="22.38" y2="96.52" width="0.152" layer="91"/>
<label x="22.38" y="96.52" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="X1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="113.03" x2="17.3" y2="113.03" width="0.152" layer="91"/>
<label x="17.3" y="113.03" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
