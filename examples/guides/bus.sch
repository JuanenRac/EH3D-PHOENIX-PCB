<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="91" name="Nets" color="2" fill="1" visible="yes" active="yes"/>
<layer number="94" name="Symbols" color="4" fill="1" visible="yes" active="yes"/>
</layers>
<schematic>
<libraries>
<library name="tutorial">
<symbols>
<symbol name="RES">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
</symbol>
</symbols>
<devicesets>
<deviceset name="R" prefix="R">
<gates>
<gate name="G$1" symbol="RES" x="0" y="0"/>
</gates>
<devices>
<device name="" package="R0805"/>
</devices>
</deviceset>
</devicesets>
</library>
</libraries>
<parts>
<part name="R1" library="tutorial" deviceset="R" device="" value="10k"/>
<part name="R2" library="tutorial" deviceset="R" device="" value="4k7"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="R1" gate="G$1" x="20.32" y="30.48" rot="R90"/>
<instance part="R2" gate="G$1" x="20.32" y="15.24" rot="R90"/>
</instances>
<busses>
<bus name="D[0..1]">
<segment>
<wire x1="0" y1="0" x2="0" y2="10" width="0.762" layer="92"/>
</segment>
</bus>
</busses>
<nets>
<net name="D0" class="0">
<segment>
<wire x1="0" y1="3" x2="10" y2="3" width="0.1524" layer="91"/>
</segment>
</net>
<net name="D1" class="0">
<segment>
<wire x1="0" y1="7" x2="10" y2="7" width="0.1524" layer="91"/>
</segment>
</net>
<net name="MID" class="0">
<segment>
<pinref part="R1" gate="G$1" pin="2"/>
<pinref part="R2" gate="G$1" pin="1"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="R2" gate="G$1" pin="2"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
