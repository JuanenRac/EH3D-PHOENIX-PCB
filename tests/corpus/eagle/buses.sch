<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="91" name="Nets" color="2" fill="1" visible="yes" active="yes"/>
<layer number="92" name="Busses" color="1" fill="1" visible="yes" active="yes"/>
</layers>
<schematic>
<sheets>
<sheet>
<busses>
<bus name="B[0..2],ENA">
<segment>
<wire x1="0" y1="0" x2="0" y2="20" width="0.762" layer="92"/>
<label x="0" y="21" size="1.778" layer="95"/>
</segment>
</bus>
<bus name="Z:Q[0..1]">
<segment>
<wire x1="50" y1="0" x2="50" y2="10" width="0.762" layer="92"/>
</segment>
</bus>
<bus name="[0..3]">
<segment>
<wire x1="80" y1="0" x2="80" y2="1" width="0.762" layer="92"/>
</segment>
</bus>
<bus name="EMPTY[0..1]">
<segment/>
</bus>
</busses>
<nets>
<net name="B1" class="0">
<segment>
<wire x1="0" y1="5" x2="10" y2="5" width="0.1524" layer="91"/>
</segment>
</net>
<net name="B2" class="0">
<segment>
<wire x1="10" y1="12" x2="0" y2="12" width="0.1524" layer="91"/>
</segment>
</net>
<net name="B0" class="0">
<segment>
<wire x1="3" y1="5" x2="10" y2="6" width="0.1524" layer="91"/>
</segment>
</net>
<net name="B7" class="0">
<segment>
<wire x1="0" y1="15" x2="10" y2="15" width="0.1524" layer="91"/>
</segment>
</net>
<net name="Q1" class="0">
<segment>
<wire x1="50" y1="2" x2="60" y2="2" width="0.1524" layer="91"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
