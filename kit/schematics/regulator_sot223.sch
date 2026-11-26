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
<package name="SOT-223">
<description>Small outline transistor with a tab, 4 pins. Sizes: JEDEC TO-261 nominal sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-2.3" y="-3.1" dx="1" dy="1.8" layer="1"/>
<smd name="2" x="0" y="-3.1" dx="1" dy="1.8" layer="1"/>
<smd name="3" x="2.3" y="-3.1" dx="1" dy="1.8" layer="1"/>
<smd name="4" x="0" y="3.1" dx="3.4" dy="1.8" layer="1"/>
<wire x1="-3.5" y1="-1.7" x2="-3.5" y2="1.7" width="0.15" layer="21"/>
<wire x1="3.5" y1="-1.7" x2="3.5" y2="1.7" width="0.15" layer="21"/>
<wire x1="-3.25" y1="-1.75" x2="3.25" y2="-1.75" width="0.1" layer="51"/>
<wire x1="3.25" y1="-1.75" x2="3.25" y2="1.75" width="0.1" layer="51"/>
<wire x1="3.25" y1="1.75" x2="-3.25" y2="1.75" width="0.1" layer="51"/>
<wire x1="-3.25" y1="1.75" x2="-3.25" y2="-1.75" width="0.1" layer="51"/>
<text x="0" y="4.3" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-5.3" size="1" layer="27">&gt;VALUE</text>
</package>
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
</packages>
<symbols>
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
<symbol name="REGULATOR">
<pin name="3" x="-7.62" y="2.54" length="short" direction="pas"/>
<pin name="1" x="-7.62" y="-2.54" length="short" direction="pas"/>
<pin name="2" x="7.62" y="2.54" length="short" direction="pas" rot="R180"/>
<pin name="4" x="7.62" y="-2.54" length="short" direction="pas" rot="R180"/>
<wire x1="-5.08" y1="-5.08" x2="5.08" y2="-5.08" width="0.254" layer="94"/>
<wire x1="5.08" y1="-5.08" x2="5.08" y2="5.08" width="0.254" layer="94"/>
<wire x1="5.08" y1="5.08" x2="-5.08" y2="5.08" width="0.254" layer="94"/>
<wire x1="-5.08" y1="5.08" x2="-5.08" y2="-5.08" width="0.254" layer="94"/>
<text x="-2.54" y="5.334" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-7.112" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
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
</symbols>
<devicesets>
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
<deviceset name="REGULATOR" prefix="U" uservalue="yes">
<description>Linear regulator, 1 ground, 2 and 4 output, 3 input (check your part)</description>
<gates>
<gate name="G$1" symbol="REGULATOR" x="0" y="0"/>
</gates>
<devices>
<device name="SOT-223" package="SOT-223">
<connects>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="4" pad="4"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
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
</devicesets>
</library>
</libraries>
<parts>
<part name="X1" library="eh3d-kit" deviceset="TERMINAL-2P" device="" value="IN"/>
<part name="C1" library="eh3d-kit" deviceset="C" device="0805" value="10u"/>
<part name="U1" library="eh3d-kit" deviceset="REGULATOR" device="SOT-223" value="REG"/>
<part name="C2" library="eh3d-kit" deviceset="C" device="0805" value="22u"/>
<part name="J1" library="eh3d-kit" deviceset="HDR-1X03" device="" value="OUT"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="X1" gate="G$1" x="30" y="27.94"/>
<instance part="C1" gate="G$1" x="30" y="43.18"/>
<instance part="U1" gate="G$1" x="30" y="60.96"/>
<instance part="C2" gate="G$1" x="30" y="78.74"/>
<instance part="J1" gate="G$1" x="30" y="96.52"/>
</instances>
<nets>
<net name="VIN" class="0">
<segment>
<pinref part="X1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="29.21" x2="17.3" y2="29.21" width="0.152" layer="91"/>
<label x="17.3" y="29.21" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="43.18" x2="37.62" y2="43.18" width="0.152" layer="91"/>
<label x="37.62" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="3"/>
<wire x1="22.38" y1="63.5" x2="19.84" y2="63.5" width="0.152" layer="91"/>
<label x="19.84" y="63.5" size="1.778" layer="95"/>
</segment>
</net>
<net name="VOUT" class="0">
<segment>
<pinref part="U1" gate="G$1" pin="2"/>
<wire x1="37.62" y1="63.5" x2="40.16" y2="63.5" width="0.152" layer="91"/>
<label x="40.16" y="63.5" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="4"/>
<wire x1="37.62" y1="58.42" x2="40.16" y2="58.42" width="0.152" layer="91"/>
<label x="40.16" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="78.74" x2="37.62" y2="78.74" width="0.152" layer="91"/>
<label x="37.62" y="78.74" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="99.06" x2="17.3" y2="99.06" width="0.152" layer="91"/>
<label x="17.3" y="99.06" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="X1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="26.67" x2="17.3" y2="26.67" width="0.152" layer="91"/>
<label x="17.3" y="26.67" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="43.18" x2="22.38" y2="43.18" width="0.152" layer="91"/>
<label x="22.38" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="1"/>
<wire x1="22.38" y1="58.42" x2="19.84" y2="58.42" width="0.152" layer="91"/>
<label x="19.84" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="78.74" x2="22.38" y2="78.74" width="0.152" layer="91"/>
<label x="22.38" y="78.74" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="96.52" x2="17.3" y2="96.52" width="0.152" layer="91"/>
<label x="17.3" y="96.52" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="93.98" x2="17.3" y2="93.98" width="0.152" layer="91"/>
<label x="17.3" y="93.98" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
