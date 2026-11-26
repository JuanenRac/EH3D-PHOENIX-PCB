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
<package name="HDR-1X04">
<description>Pin header, 1 row of 4, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-3.81" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-1.27" y="0" drill="1" diameter="1.7"/>
<pad name="3" x="1.27" y="0" drill="1" diameter="1.7"/>
<pad name="4" x="3.81" y="0" drill="1" diameter="1.7"/>
<wire x1="-5.08" y1="-1.27" x2="5.08" y2="-1.27" width="0.15" layer="21"/>
<wire x1="5.08" y1="-1.27" x2="5.08" y2="1.27" width="0.15" layer="21"/>
<wire x1="5.08" y1="1.27" x2="-5.08" y2="1.27" width="0.15" layer="21"/>
<wire x1="-5.08" y1="1.27" x2="-5.08" y2="-1.27" width="0.15" layer="21"/>
<text x="0" y="1.6" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.6" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="SOIC-8">
<description>Small outline package, 8 pins, pitch 1.27 mm, body 3.9 mm. Sizes: JEDEC MS-012 nominal sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-2.7" y="1.905" dx="1.55" dy="0.6" layer="1"/>
<smd name="2" x="-2.7" y="0.635" dx="1.55" dy="0.6" layer="1"/>
<smd name="3" x="-2.7" y="-0.635" dx="1.55" dy="0.6" layer="1"/>
<smd name="4" x="-2.7" y="-1.905" dx="1.55" dy="0.6" layer="1"/>
<smd name="5" x="2.7" y="-1.905" dx="1.55" dy="0.6" layer="1"/>
<smd name="6" x="2.7" y="-0.635" dx="1.55" dy="0.6" layer="1"/>
<smd name="7" x="2.7" y="0.635" dx="1.55" dy="0.6" layer="1"/>
<smd name="8" x="2.7" y="1.905" dx="1.55" dy="0.6" layer="1"/>
<wire x1="-1.95" y1="2.555" x2="1.95" y2="2.555" width="0.15" layer="21"/>
<wire x1="-1.95" y1="-2.555" x2="1.95" y2="-2.555" width="0.15" layer="21"/>
<circle x="-3.925" y="1.905" radius="0.15" width="0.15" layer="21"/>
<wire x1="-1.95" y1="-2.355" x2="1.95" y2="-2.355" width="0.1" layer="51"/>
<wire x1="1.95" y1="-2.355" x2="1.95" y2="2.355" width="0.1" layer="51"/>
<wire x1="1.95" y1="2.355" x2="-1.95" y2="2.355" width="0.1" layer="51"/>
<wire x1="-1.95" y1="2.355" x2="-1.95" y2="-2.355" width="0.1" layer="51"/>
<text x="0" y="2.855" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-3.855" size="1" layer="27">&gt;VALUE</text>
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
<symbol name="PINS-1X04">
<pin name="1" x="-10.16" y="3.81" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="-1.27" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="-3.81" length="short" direction="pas"/>
<wire x1="-7.62" y1="-6.35" x2="7.62" y2="-6.35" width="0.254" layer="94"/>
<wire x1="7.62" y1="-6.35" x2="7.62" y2="6.35" width="0.254" layer="94"/>
<wire x1="7.62" y1="6.35" x2="-7.62" y2="6.35" width="0.254" layer="94"/>
<wire x1="-7.62" y1="6.35" x2="-7.62" y2="-6.35" width="0.254" layer="94"/>
<text x="-2.54" y="6.604" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-8.382" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="IC-8">
<pin name="1" x="-10.16" y="3.81" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="-1.27" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="-3.81" length="short" direction="pas"/>
<pin name="8" x="10.16" y="3.81" length="short" direction="pas" rot="R180"/>
<pin name="7" x="10.16" y="1.27" length="short" direction="pas" rot="R180"/>
<pin name="6" x="10.16" y="-1.27" length="short" direction="pas" rot="R180"/>
<pin name="5" x="10.16" y="-3.81" length="short" direction="pas" rot="R180"/>
<wire x1="-7.62" y1="-6.35" x2="7.62" y2="-6.35" width="0.254" layer="94"/>
<wire x1="7.62" y1="-6.35" x2="7.62" y2="6.35" width="0.254" layer="94"/>
<wire x1="7.62" y1="6.35" x2="-7.62" y2="6.35" width="0.254" layer="94"/>
<wire x1="-7.62" y1="6.35" x2="-7.62" y2="-6.35" width="0.254" layer="94"/>
<text x="-2.54" y="6.604" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-8.382" size="1.778" layer="96">&gt;VALUE</text>
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
<deviceset name="HDR-1X04" prefix="J" uservalue="yes">
<description>Pin header 1 x 4</description>
<gates>
<gate name="G$1" symbol="PINS-1X04" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X04">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="4" pad="4"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
</devices>
</deviceset>
<deviceset name="IC8" prefix="U" uservalue="yes">
<description>Generic integrated circuit, 8 pins</description>
<gates>
<gate name="G$1" symbol="IC-8" x="0" y="0"/>
</gates>
<devices>
<device name="SOIC" package="SOIC-8">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="4" pad="4"/>
<connect gate="G$1" pin="8" pad="8"/>
<connect gate="G$1" pin="7" pad="7"/>
<connect gate="G$1" pin="6" pad="6"/>
<connect gate="G$1" pin="5" pad="5"/>
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
<part name="J1" library="eh3d-kit" deviceset="HDR-1X04" device="" value="LEFT"/>
<part name="U1" library="eh3d-kit" deviceset="IC8" device="SOIC" value="GENERIC8"/>
<part name="C1" library="eh3d-kit" deviceset="C" device="0603" value="100n"/>
<part name="J2" library="eh3d-kit" deviceset="HDR-1X04" device="" value="RIGHT"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="J1" gate="G$1" x="30" y="30.48"/>
<instance part="U1" gate="G$1" x="30" y="53.34"/>
<instance part="C1" gate="G$1" x="30" y="71.12"/>
<instance part="J2" gate="G$1" x="30" y="91.44"/>
</instances>
<nets>
<net name="S1" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="34.29" x2="17.3" y2="34.29" width="0.152" layer="91"/>
<label x="17.3" y="34.29" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="57.15" x2="17.3" y2="57.15" width="0.152" layer="91"/>
<label x="17.3" y="57.15" size="1.778" layer="95"/>
</segment>
</net>
<net name="S2" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="31.75" x2="17.3" y2="31.75" width="0.152" layer="91"/>
<label x="17.3" y="31.75" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="54.61" x2="17.3" y2="54.61" width="0.152" layer="91"/>
<label x="17.3" y="54.61" size="1.778" layer="95"/>
</segment>
</net>
<net name="S3" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="29.21" x2="17.3" y2="29.21" width="0.152" layer="91"/>
<label x="17.3" y="29.21" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="52.07" x2="17.3" y2="52.07" width="0.152" layer="91"/>
<label x="17.3" y="52.07" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="26.67" x2="17.3" y2="26.67" width="0.152" layer="91"/>
<label x="17.3" y="26.67" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="U1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="49.53" x2="17.3" y2="49.53" width="0.152" layer="91"/>
<label x="17.3" y="49.53" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="71.12" x2="22.38" y2="71.12" width="0.152" layer="91"/>
<label x="22.38" y="71.12" size="1.778" layer="95"/>
</segment>
</net>
<net name="S5" class="0">
<segment>
<pinref part="U1" gate="G$1" pin="5"/>
<wire x1="40.16" y1="49.53" x2="42.7" y2="49.53" width="0.152" layer="91"/>
<label x="42.7" y="49.53" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="1"/>
<wire x1="19.84" y1="95.25" x2="17.3" y2="95.25" width="0.152" layer="91"/>
<label x="17.3" y="95.25" size="1.778" layer="95"/>
</segment>
</net>
<net name="S6" class="0">
<segment>
<pinref part="U1" gate="G$1" pin="6"/>
<wire x1="40.16" y1="52.07" x2="42.7" y2="52.07" width="0.152" layer="91"/>
<label x="42.7" y="52.07" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="2"/>
<wire x1="19.84" y1="92.71" x2="17.3" y2="92.71" width="0.152" layer="91"/>
<label x="17.3" y="92.71" size="1.778" layer="95"/>
</segment>
</net>
<net name="S7" class="0">
<segment>
<pinref part="U1" gate="G$1" pin="7"/>
<wire x1="40.16" y1="54.61" x2="42.7" y2="54.61" width="0.152" layer="91"/>
<label x="42.7" y="54.61" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="3"/>
<wire x1="19.84" y1="90.17" x2="17.3" y2="90.17" width="0.152" layer="91"/>
<label x="17.3" y="90.17" size="1.778" layer="95"/>
</segment>
</net>
<net name="VCC" class="0">
<segment>
<pinref part="U1" gate="G$1" pin="8"/>
<wire x1="40.16" y1="57.15" x2="42.7" y2="57.15" width="0.152" layer="91"/>
<label x="42.7" y="57.15" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="71.12" x2="37.62" y2="71.12" width="0.152" layer="91"/>
<label x="37.62" y="71.12" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J2" gate="G$1" pin="4"/>
<wire x1="19.84" y1="87.63" x2="17.3" y2="87.63" width="0.152" layer="91"/>
<label x="17.3" y="87.63" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
