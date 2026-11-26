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
<package name="HDR-1X02">
<description>Pin header, 1 row of 2, pitch 2.54 mm. Sizes: 0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1.27" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="1.27" y="0" drill="1" diameter="1.7"/>
<wire x1="-2.54" y1="-1.27" x2="2.54" y2="-1.27" width="0.15" layer="21"/>
<wire x1="2.54" y1="-1.27" x2="2.54" y2="1.27" width="0.15" layer="21"/>
<wire x1="2.54" y1="1.27" x2="-2.54" y2="1.27" width="0.15" layer="21"/>
<wire x1="-2.54" y1="1.27" x2="-2.54" y2="-1.27" width="0.15" layer="21"/>
<text x="0" y="1.6" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-2.6" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="C-ELEC-D5">
<description>Radial electrolytic capacitor 5 mm class, pad 1 is positive. Sizes: IEC 60384-4 usual can sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1" y="0" drill="0.7" diameter="1.3" shape="square"/>
<pad name="2" x="1" y="0" drill="0.7" diameter="1.3"/>
<circle x="0" y="0" radius="2.75" width="0.15" layer="21"/>
<text x="0" y="3.05" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-4.05" size="1" layer="27">&gt;VALUE</text>
</package>
<package name="C-ELEC-D8">
<description>Radial electrolytic capacitor 8 mm class, pad 1 is positive. Sizes: IEC 60384-4 usual can sizes; pads worked out for this kit. Not verified against a datasheet.</description>
<pad name="1" x="-1.75" y="0" drill="0.8" diameter="1.6" shape="square"/>
<pad name="2" x="1.75" y="0" drill="0.8" diameter="1.6"/>
<circle x="0" y="0" radius="4.25" width="0.15" layer="21"/>
<text x="0" y="4.55" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-5.55" size="1" layer="27">&gt;VALUE</text>
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
<circle x="0" y="0" radius="2.2" width="0.15" layer="21"/>
<text x="0" y="2.5" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-3.5" size="1" layer="27">&gt;VALUE</text>
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
<symbol name="PINS-1X02">
<pin name="1" x="-10.16" y="1.27" length="short" direction="pas"/>
<pin name="2" x="-10.16" y="-1.27" length="short" direction="pas"/>
<wire x1="-7.62" y1="-3.81" x2="7.62" y2="-3.81" width="0.254" layer="94"/>
<wire x1="7.62" y1="-3.81" x2="7.62" y2="3.81" width="0.254" layer="94"/>
<wire x1="7.62" y1="3.81" x2="-7.62" y2="3.81" width="0.254" layer="94"/>
<wire x1="-7.62" y1="3.81" x2="-7.62" y2="-3.81" width="0.254" layer="94"/>
<text x="-2.54" y="4.064" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-5.842" size="1.778" layer="96">&gt;VALUE</text>
</symbol>
<symbol name="CAPACITOR-POL">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
<wire x1="-2.54" y1="0" x2="-0.508" y2="0" width="0.152" layer="94"/>
<wire x1="-0.508" y1="-1.524" x2="-0.508" y2="1.524" width="0.406" layer="94"/>
<wire x1="0.508" y1="-1.524" x2="0.508" y2="1.524" width="0.406" layer="94"/>
<wire x1="0.508" y1="0" x2="2.54" y2="0" width="0.152" layer="94"/>
<wire x1="-2.032" y1="1.016" x2="-1.016" y2="1.016" width="0.152" layer="94"/>
<wire x1="-1.524" y1="0.508" x2="-1.524" y2="1.524" width="0.152" layer="94"/>
<text x="-2.54" y="2.54" size="1.778" layer="95">&gt;NAME</text>
<text x="-2.54" y="-4.318" size="1.778" layer="96">&gt;VALUE</text>
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
<deviceset name="HDR-1X02" prefix="J" uservalue="yes">
<description>Pin header 1 x 2</description>
<gates>
<gate name="G$1" symbol="PINS-1X02" x="0" y="0"/>
</gates>
<devices>
<device name="" package="HDR-1X02">
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
<deviceset name="C-POL" prefix="C" uservalue="yes">
<description>Electrolytic capacitor, pin 1 positive</description>
<gates>
<gate name="G$1" symbol="CAPACITOR-POL" x="0" y="0"/>
</gates>
<devices>
<device name="D5" package="C-ELEC-D5">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="D8" package="C-ELEC-D8">
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
<part name="J1" library="eh3d-kit" deviceset="HDR-1X02" device="" value="PWR"/>
<part name="C1" library="eh3d-kit" deviceset="C-POL" device="D5" value="10u"/>
<part name="R1" library="eh3d-kit" deviceset="R" device="AXIAL-10.16" value="470"/>
<part name="R2" library="eh3d-kit" deviceset="R" device="AXIAL-7.62" value="470"/>
<part name="D1" library="eh3d-kit" deviceset="LED" device="3MM" value="YELLOW"/>
<part name="D2" library="eh3d-kit" deviceset="LED" device="5MM" value="BLUE"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="J1" gate="G$1" x="30" y="27.94"/>
<instance part="C1" gate="G$1" x="30" y="43.18"/>
<instance part="R1" gate="G$1" x="30" y="58.42"/>
<instance part="R2" gate="G$1" x="30" y="73.66"/>
<instance part="D1" gate="G$1" x="30" y="88.9"/>
<instance part="D2" gate="G$1" x="30" y="104.14"/>
</instances>
<nets>
<net name="VCC" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="1"/>
<wire x1="19.84" y1="29.21" x2="17.3" y2="29.21" width="0.152" layer="91"/>
<label x="17.3" y="29.21" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="43.18" x2="22.38" y2="43.18" width="0.152" layer="91"/>
<label x="22.38" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="58.42" x2="22.38" y2="58.42" width="0.152" layer="91"/>
<label x="22.38" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="R2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="73.66" x2="22.38" y2="73.66" width="0.152" layer="91"/>
<label x="22.38" y="73.66" size="1.778" layer="95"/>
</segment>
</net>
<net name="A1" class="0">
<segment>
<pinref part="R1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="58.42" x2="37.62" y2="58.42" width="0.152" layer="91"/>
<label x="37.62" y="58.42" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="88.9" x2="37.62" y2="88.9" width="0.152" layer="91"/>
<label x="37.62" y="88.9" size="1.778" layer="95"/>
</segment>
</net>
<net name="A2" class="0">
<segment>
<pinref part="R2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="73.66" x2="37.62" y2="73.66" width="0.152" layer="91"/>
<label x="37.62" y="73.66" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D2" gate="G$1" pin="2"/>
<wire x1="35.08" y1="104.14" x2="37.62" y2="104.14" width="0.152" layer="91"/>
<label x="37.62" y="104.14" size="1.778" layer="95"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="J1" gate="G$1" pin="2"/>
<wire x1="19.84" y1="26.67" x2="17.3" y2="26.67" width="0.152" layer="91"/>
<label x="17.3" y="26.67" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="C1" gate="G$1" pin="2"/>
<wire x1="35.08" y1="43.18" x2="37.62" y2="43.18" width="0.152" layer="91"/>
<label x="37.62" y="43.18" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D1" gate="G$1" pin="1"/>
<wire x1="24.92" y1="88.9" x2="22.38" y2="88.9" width="0.152" layer="91"/>
<label x="22.38" y="88.9" size="1.778" layer="95"/>
</segment>
<segment>
<pinref part="D2" gate="G$1" pin="1"/>
<wire x1="24.92" y1="104.14" x2="22.38" y2="104.14" width="0.152" layer="91"/>
<label x="22.38" y="104.14" size="1.778" layer="95"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
