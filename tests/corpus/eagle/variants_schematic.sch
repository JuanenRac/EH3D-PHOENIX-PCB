<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="91" name="Nets" color="2" fill="1" visible="yes" active="yes"/>
<layer number="94" name="Symbols" color="4" fill="1" visible="yes" active="yes"/>
<layer number="95" name="Names" color="7" fill="1" visible="yes" active="yes"/>
</layers>
<schematic>
<description>Variants, attributes and a module</description>
<libraries>
<library name="hand"/>
</libraries>
<attributes>
<attribute name="DRAWN" value="by hand" constant="no"/>
</attributes>
<variantdefs>
<variantdef name="FULL" current="yes"/>
<variantdef name="LITE"/>
</variantdefs>
<classes>
<class number="0" name="default" width="0" drill="0"/>
</classes>
<modules>
<module name="PSU" prefix="P_" dx="30" dy="20">
<ports>
<port name="VIN" side="left" coord="5" direction="in"/>
</ports>
<variantdefs>
<variantdef name="A"/>
</variantdefs>
<parts/>
<sheets>
<sheet/>
</sheets>
</module>
</modules>
<parts>
<part name="R1" library="hand" deviceset="R" device="" value="10k">
<attribute name="MPN" value="RC0805FR-0710KL" x="0" y="0" size="1.27" layer="96" display="off"/>
<variant name="LITE" populate="no"/>
</part>
<part name="R2" library="hand" deviceset="R" device="" value="4k7">
<variant name="LITE" value="10k"/>
</part>
</parts>
<sheets>
<sheet>
<plain/>
<instances/>
<busses/>
<nets>
<net name="N$1" class="0">
<segment>
<wire x1="0" y1="0" x2="5" y2="0" width="0.1524" layer="91"/>
</segment>
</net>
</nets>
<moduleinsts>
<moduleinst name="PSU1" module="PSU" modulevariant="A" x="0" y="0" offset="0"/>
</moduleinsts>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
