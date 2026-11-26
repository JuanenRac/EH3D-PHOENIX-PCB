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
<package name="HDR-1X05">
<description>Pin header, 1 row of 5, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-5.08" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="-2.54" y="0" drill="1" diameter="1.7"/>
<pad name="3" x="0" y="0" drill="1" diameter="1.7"/>
<pad name="4" x="2.54" y="0" drill="1" diameter="1.7"/>
<pad name="5" x="5.08" y="0" drill="1" diameter="1.7"/>
<wire x1="-6.35" y1="-1.27" x2="6.35" y2="-1.27" width="0.15" layer="21"/>
<wire x1="6.35" y1="-1.27" x2="6.35" y2="1.27" width="0.15" layer="21"/>
<wire x1="6.35" y1="1.27" x2="-6.35" y2="1.27" width="0.15" layer="21"/>
<wire x1="-6.35" y1="1.27" x2="-6.35" y2="-1.27" width="0.15" layer="21"/>
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
<package name="LED-0603">
<description>Chip LED 0603, pad 1 is the cathode (marked). Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.8" y="0" dx="0.9" dy="0.95" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.9" dy="0.95" layer="1"/>
<wire x1="-0.35" y1="0.775" x2="0.35" y2="0.775" width="0.15" layer="21"/>
<wire x1="-0.35" y1="-0.775" x2="0.35" y2="-0.775" width="0.15" layer="21"/>
<wire x1="-1.6" y1="-0.775" x2="-1.6" y2="0.775" width="0.15" layer="21"/>
<wire x1="-0.8" y1="-0.4" x2="0.8" y2="-0.4" width="0.1" layer="51"/>
<wire x1="0.8" y1="-0.4" x2="0.8" y2="0.4" width="0.1" layer="51"/>
<wire x1="0.8" y1="0.4" x2="-0.8" y2="0.4" width="0.1" layer="51"/>
<wire x1="-0.8" y1="0.4" x2="-0.8" y2="-0.4" width="0.1" layer="51"/>
<text x="0" y="1.075" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.075" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="LED-0805">
<description>Chip LED 0805, pad 1 is the cathode (marked). Sizes: EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit. Not verified against a datasheet.</description>
<smd name="1" x="-0.95" y="0" dx="1.15" dy="1.45" layer="1"/>
<smd name="2" x="0.95" y="0" dx="1.15" dy="1.45" layer="1"/>
<wire x1="-0.375" y1="1.025" x2="0.375" y2="1.025" width="0.15" layer="21"/>
<wire x1="-0.375" y1="-1.025" x2="0.375" y2="-1.025" width="0.15" layer="21"/>
<wire x1="-1.875" y1="-1.025" x2="-1.875" y2="1.025" width="0.15" layer="21"/>
<wire x1="-1" y1="-0.625" x2="1" y2="-0.625" width="0.1" layer="51"/>
<wire x1="1" y1="-0.625" x2="1" y2="0.625" width="0.1" layer="51"/>
<wire x1="1" y1="0.625" x2="-1" y2="0.625" width="0.1" layer="51"/>
<wire x1="-1" y1="0.625" x2="-1" y2="-0.625" width="0.1" layer="51"/>
<text x="0" y="1.325" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.325" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="LED-THT-3MM">
<description>Through hole LED 3mm, lead pitch 2.54 mm, pad 1 is the cathode. Sizes: T-1 and T-1 3/4 lamp sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1.27" y="0" drill="0.8" diameter="1.6" shape="square"/>
<pad name="2" x="1.27" y="0" drill="0.8" diameter="1.6"/>
<circle x="0" y="0" radius="2.5" width="0.15" layer="21"/>
<text x="0" y="2.8" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-3.8" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="LED-THT-5MM">
<description>Through hole LED 5mm, lead pitch 2.54 mm, pad 1 is the cathode. Sizes: T-1 and T-1 3/4 lamp sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1.27" y="0" drill="0.9" diameter="1.8" shape="square"/>
<pad name="2" x="1.27" y="0" drill="0.9" diameter="1.8"/>
<circle x="0" y="0" radius="3.2" width="0.15" layer="21"/>
<text x="0" y="3.5" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-4.5" size="1" layer="27">&gt;VALUE</text>
</package>
</packages>
<symbols>
<symbol name="PINS-1X05">
<pin name="1" x="-10.16" y="5.08" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="2.54" length="short" direction="pas"/>
<pin name="3" x="-10.16" y="0" length="short" direction="pas"/>
<pin name="4" x="-10.16" y="-2.54" length="short" direction="pas"/>
<pin name="5" x="-10.16" y="-5.08" length="short" direction="pas"/>
<wire x1="-7.62" y1="-7.62" x2="7.62" y2="-7.62" width="0.254" layer="94"/>
<wire x1="7.62" y1="-7.62" x2="7.62" y2="7.62" width="0.254" layer="94"/>
<wire x1="7.62" y1="7.62" x2="-7.62" y2="7.62" width="0.254" layer="94"/>
<wire x1="-7.62" y1="7.62" x2="-7.62" y2="-7.62" width="0.254" layer="94"/>
<text x="-2.54" y="7.874" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-9.652" size="1.778" layer="96">&gt;VALUE</text>
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
<symbol name="LED">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="0" x2="2.54" y2="0" width="0.152" layer="94"/>
<wire x1="-1.27" y1="-1.27" x2="-1.27" y2="1.27" width="0.254" layer="94"/>
<wire x1="1.27" y1="-1.27" x2="1.27" y2="1.27" width="0.254" layer="94"/>
<wire x1="1.27" y1="1.27" x2="-1.27" y2="0" width="0.254" layer="94"/>
<wire x1="-1.27" y1="0" x2="1.27" y2="-1.27" width="0.254" layer="94"/>
<wire x1="0" y1="1.778" x2="1.016" y2="2.794" width="0.152" layer="94"/>
<wire x1="1.27" y1="1.524" x2="2.286" y2="2.54" width="0.152" layer="94"/>
<text x="-2.54" y="2.54" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-4.318" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
</symbols>
<devicesets>
<deviceset name="HDR-1X05" prefix="J" uservalue="yes">
<description>Pin header 1 x 5</description>
<gates>
<gate name="G$1" symbol="PINS-1X05" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X05">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
<connect gate="G$1" pin="3" pad="3"/>
<connect gate="G$1" pin="4" pad="4"/>
<connect gate="G$1" pin="5" pad="5"/>
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
<deviceset name="LED" prefix="D" uservalue="yes">
<description>LED, pin 1 cathode, pin 2 anode</description>
<gates>
<gate name="G$1" symbol="LED" x="0" y="0"/>
</gates>
<devices>
<device name="0603" package="LED-0603">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="0805" package="LED-0805">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="3MM" package="LED-THT-3MM">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="5MM" package="LED-THT-5MM">
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
<part name="J1" library="eh3d-kit" deviceset="HDR-1X05" device="" value="LEDS"/>
<part name="R1" library="eh3d-kit" deviceset="R" device="0603" value="330"/>
<part name="R2" library="eh3d-kit" deviceset="R" device="0603" value="330"/>
<part name="R3" library="eh3d-kit" deviceset="R" device="0603" value="330"/>
<part name="R4" library="eh3d-kit" deviceset="R" device="0603" value="330"/>
<part name="D1" library="eh3d-kit" deviceset="LED" device="0603" value="RED"/>
<part name="D2" library="eh3d-kit" deviceset="LED" device="0603" value="RED"/>
<part name="D3" library="eh3d-kit" deviceset="LED" device="0603" value="RED"/>
<part name="D4" library="eh3d-kit" deviceset="LED" device="0603" value="RED"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="J1" gate="G$1" x="30" y="30.48"/>
<instance part="R1" gate="G$1" x="30" y="50.8"/>
<instance part="R2" gate="G$1" x="30" y="66.04"/>
<instance part="R3" gate="G$1" x="30" y="81.28"/>
<instance part="R4" gate="G$1" x="30" y="96.52"/>
<instance part="D1" gate="G$1" x="30" y="111.76"/>
<instance part="D2" gate="G$1" x="30" y="127"/>
<instance part="D3" gate="G$1" x="30" y="142.24"/>
<instance part="D4" gate="G$1" x="30" y="157.48"/>
</instances>
<nets>
<net name="IN1" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="35.56" x2="17.3" y2="35.56" width="0.152" layer="91"/>
<label x="17.3" y="35.56" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="50.8" x2="22.38" y2="50.8" width="0.152" layer="91"/>
<label x="22.38" y="50.8" size="1.778" layer="95"/>
</segment>
</net>
<net name="IN2" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="33.02" x2="17.3" y2="33.02" width="0.152" layer="91"/>
<label x="17.3" y="33.02" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="66.04" x2="22.38" y2="66.04" width="0.152" layer="91"/>
<label x="22.38" y="66.04" size="1.778" layer="95"/>
</segment>
</net>
<net name="IN3" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="3"/>
<wire x1="19.84" y1="30.48" x2="17.3" y2="30.48" width="0.152" layer="91"/>
<label x="17.3" y="30.48" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R3" gate="G$1" pin="1"/>
<wire x1="24.92" y1="81.28" x2="22.38" y2="81.28" width="0.152" layer="91"/>
<label x="22.38" y="81.28" size="1.778" layer="95"/>
</segment>
</net>
<net name="IN4" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="4"/>
<wire x1="19.84" y1="27.94" x2="17.3" y2="27.94" width="0.152" layer="91"/>
<label x="17.3" y="27.94" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R4" gate="G$1" pin="1"/>
<wire x1="24.92" y1="96.52" x2="22.38" y2="96.52" width="0.152" layer="91"/>
<label x="22.38" y="96.52" size="1.778" layer="95"/>
</segment>
</net>
<net name="A1" class="0">
<segment>
<pinref part="R1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="50.8" x2="37.62" y2="50.8" width="0.152" layer="91"/>
<label x="37.62" y="50.8" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="111.76" x2="37.62" y2="111.76" width="0.152" layer="91"/>
<label x="37.62" y="111.76" size="1.778" layer="95"/>
</segment>
</net>
<net name="A2" class="0">
<segment>
<pinref part="R2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="66.04" x2="37.62" y2="66.04" width="0.152" layer="91"/>
<label x="37.62" y="66.04" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="127" x2="37.62" y2="127" width="0.152" layer="91"/>
<label x="37.62" y="127" size="1.778" layer="95"/>
</segment>
</net>
<net name="A3" class="0">
<segment>
<pinref part="R3" gate="G$1" pin="2"/>
<wire x1="35.08" y1="81.28" x2="37.62" y2="81.28" width="0.152" layer="91"/>
<label x="37.62" y="81.28" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D3" gate="G$1" pin="2"/>
<wire x1="35.08" y1="142.24" x2="37.62" y2="142.24" width="0.152" layer="91"/>
<label x="37.62" y="142.24" size="1.778" layer="95"/>
</segment>
</net>
<net name="A4" class="0">
<segment>
<pinref part="R4" gate="G$1" pin="2"/>
<wire x1="35.08" y1="96.52" x2="37.62" y2="96.52" width="0.152" layer="91"/>
<label x="37.62" y="96.52" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D4" gate="G$1" pin="2"/>
<wire x1="35.08" y1="157.48" x2="37.62" y2="157.48" width="0.152" layer="91"/>
<label x="37.62" y="157.48" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="D1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="111.76" x2="22.38" y2="111.76" width="0.152" layer="91"/>
<label x="22.38" y="111.76" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="127" x2="22.38" y2="127" width="0.152" layer="91"/>
<label x="22.38" y="127" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D3" gate="G$1" pin="1"/>
<wire x1="24.92" y1="142.24" x2="22.38" y2="142.24" width="0.152" layer="91"/>
<label x="22.38" y="142.24" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D4" gate="G$1" pin="1"/>
<wire x1="24.92" y1="157.48" x2="22.38" y2="157.48" width="0.152" layer="91"/>
<label x="22.38" y="157.48" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="J1" gate="G$1" pin="5"/>
<wire x1="19.84" y1="25.4" x2="17.3" y2="25.4" width="0.152" layer="91"/>
<label x="17.3" y="25.4" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
