# Figures to add to the materials database

The quick library of laminates of the stack-up crate used to keep its own figures. It now reads everything from the materials database of the suite (`eh3d-materials`), so the figures below, which the earlier list had and the database lacks or states differently, are to be offered to that database. Nothing was changed there. The proposed identifiers follow the form `maker/name`; every figure is a sheet value and is not verified.

| Proposed id | Figure | Earlier list | In the database | Note |
|---|---|---|---|---|
| `rogers/ro4003c` | glass transition | over 280 C (lower bound) | none | the database has no lower bound; add the figure with a note that the sheet says "over 280" |
| `rogers/ro4350b` | glass transition | over 280 C (lower bound) | none | the same |
| `taconic/rf-35` | glass transition | over 315 C (lower bound) | none | the same |
| `arlon/85n` | glass transition | over 250 C (lower bound) | 250 C (TMA) | the same figure; the database states it as a measured value |
| `dupont/pyralux-ap-9111` | glass transition | 220 C | none | the sheet of the family states it |
| `isola/is400` | expansion in Y | 13 ppm | none | the sheet states one value for X and Y |
| `thinflex/w-0505ed-n4` | expansion in X and Y | 24 ppm | none | the sheet of the film states it |
| `taconic/tlc-32` | thermal conductivity | 0.24 W/mK | 0.24 W/mK | agrees |
| `iteq/it-158tc` | glass transition | 150 C (product IT-158) | 155 C (product IT-158TC) | two products: add `iteq/it-158` with 150 C |
| `iteq/it-158` | the product itself | present | missing | add the record for the plain IT-158 sheet |
| `copper-foil` | conductivity | 59.59 MS/m | no figure | a physical constant; the library keeps it in `config/laminate_links.json` |
| `taconic/tlx-9` | thermal conductivity | none | 0.19 W/mK | the database is richer; nothing to add |
| `rogers/ro3003`, `rogers/ro3006`, `rogers/ro3010` | decomposition | none | 500 C | the database is richer; nothing to add |

Where the earlier list and the database give a different figure for the same product at the same frequency, the database wins in the library and the difference is in the table. The points the library reads are fixed in `config/laminate_links.json`; a record that has no point at that frequency gives no constant or no loss and therefore no sample.
