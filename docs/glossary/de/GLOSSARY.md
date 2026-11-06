# Glossar

Die Begriffe des Fachgebiets, wie dieses Programm sie verwendet. Die Tabelle am Ende der englischen Seite nennt jeden Begriff in den acht Sprachen.

| Begriff | Bedeutung |
|---|---|
| **nm (Nanometer)** | Die Einheit jeder Länge im Modell, als ganze Zahl gespeichert: 1 mm sind 1 000 000 nm, daher haben Summen und Vergleiche keinen Rundungsfehler. |
| **Netz** | Alle Pads, Leiterbahnen und Vias, die elektrisch verbunden sein müssen; es hat einen Namen wie GND. |
| **Netzklasse** | Eine Gruppe von Netzen mit gemeinsamen Regeln: Bahnbreite, Abstand und Via-Größe. Die Regeldatei des zweiten Formats enthält die Klassen. |
| **Via (Durchkontaktierung)** | Eine durchkontaktierte Bohrung, die das Kupfer zweier Lagen verbindet. |
| **Restring** | Der Kupferring, der um eine Bohrung bleibt; ist er zu dünn, kann der Bohrer ihn durchbrechen. |
| **Footprint (Bauform)** | Die Kupfer-Pads, Bohrungen und Zeichnungen eines Bauteils auf der Platine. |
| **Courtyard (Platzbedarf)** | Der Bereich um einen Footprint, in den kein anderes Bauteil ragen darf. |
| **Kupferfläche** | Eine gefüllte Kupferfläche, meist eine Massefläche, die ihren Abstand zu anderen Netzen hält. |
| **Lagenaufbau** | Reihenfolge, Dicke und Material der Kupfer- und Isolierlagen einer Platine; die Impedanz einer Leitung hängt davon ab. |
| **DRC (Entwurfsregelprüfung)** | Die Prüfung einer Platine gegen Grenzen wie schmalste Bahn, Abstand, Bohrung und Restring. Befehl: `phoenix check`. |
| **ERC (elektrische Regelprüfung)** | Die Prüfung eines Schaltplans: Pins an keinem Netz, Eingänge ohne Treiber, Versorgungen ohne Quelle usw. Befehl: `phoenix erc`. |
| **Luftlinien (Ratsnest)** | Die geraden Linien, die zeigen, welche Verbindungen noch zu verlegen sind. Befehl: `phoenix ratsnest`. |
| **Gerber X2** | Das Format der Dateien für Kupfer, Lötstopplack, Paste und Bestückungsdruck, das der Hersteller liest; X2 ergänzt Attribute wie die Funktion der Lage. |
| **Excellon** | Das Format der Bohrdateien: Werkzeuge und Positionen der Bohrungen, durchkontaktiert oder nicht. |
| **DSN / SES** | Die Specctra-Dateien für einen externen Autorouter: DSN ist der Entwurf, der hinausgeht, SES die Sitzung mit den Routen, die zurückkommt. Befehl: `phoenix autoroute`. |
| **Ritzen (V-Nut)** | Eine gerade Nut zwischen zwei Platinen eines Nutzens, damit man sie auseinanderbrechen kann. |
| **Nutzen** | Mehrere Platinen gemeinsam auf einer Tafel mit Rahmen, Stegen, Passmarken und Werkzeuglöchern, um sie zugleich zu fertigen und zu bestücken. Befehl: `phoenix panel`. |
| **Stege mit Perforation (Mouse Bites)** | Ein Steg zwischen zwei Platinen mit einer Reihe kleiner Löcher, damit er sauber bricht. |
| **Passmarke (Fiducial)** | Eine kleine runde Marke aus blankem Kupfer, an der der Bestückungsautomat die Lage der Platine erkennt. |
| **Bestückungsdaten (Pick and Place)** | Die Datei mit Referenz, Position, Drehung und Seite jedes Bauteils für den Bestückungsautomaten. Befehl: `phoenix pnp`. |
| **Schablone (Stencil)** | Das dünne Blech mit Öffnungen, durch die die Lotpaste auf die Pads gestrichen wird. Befehl: `phoenix stencil`. |
| **BOM (Stückliste)** | Die Liste der Bauteile einer Platine, nach Wert und Footprint gruppiert, mit Mengen und Preisen. Befehl: `phoenix bom`. |
| **Differenzialpaar** | Zwei Leiterbahnen, die ein Signal als Differenz ihrer Spannungen führen; sie müssen Abstand, gleiche Längen und Symmetrie halten. Befehl: `phoenix pairs`. |
| **Übersprechen (NEXT, FEXT)** | Der Teil eines Signals, der auf einer Nachbarbahn erscheint; nah (NEXT) am Treiber, fern (FEXT) am anderen Ende. Befehl: `phoenix si crosstalk`. |
| **S-Parameter / Touchstone** | Reflexion und Transmission eines Netzwerks bei jeder Frequenz; Touchstone (.s1p, .s2p) ist die Datei, die sie speichert. Befehl: `phoenix rf touchstone`. |
| **Stehwellenverhältnis (VSWR)** | Eine Zahl ab 1, die sagt, wie schlecht eine Last an eine Leitung angepasst ist: 1 ist perfekte Anpassung. |
| **Monte Carlo** | Viele Läufe einer Schaltung, bei denen die Werte zufällig innerhalb ihrer Toleranzen schwanken; ein fester Startwert ergibt stets dieselbe Tabelle. |
| **Netzliste** | Der Text, der die Elemente einer Schaltung und die Knoten, die sie verbinden, auflistet; er wird vom Simulator gelesen. Befehl: `phoenix netlist`. |
| **Probelauf** | Ein Lauf, der alles tut außer zu schreiben: Er zeigt, was geschrieben oder geändert würde. Fast jeder Befehl ist ein Probelauf, außer `--out` nennt eine neue Datei. |

* [English](../../GLOSSARY.md)
