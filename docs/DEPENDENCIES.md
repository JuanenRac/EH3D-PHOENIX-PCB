# EH3D-PHOENIX-PCB: dependencies

One row per relation with another repository. The kind is conceptual, compile-time or run-time. The state of a contract is: not written, drafted, accepted, frozen or published. A repository existing does not mean that its contract is ready: this project does not implement or consume a contract that is not accepted.

| From | To | Kind | Contract | Contract state | Minimum version | Tested version | Compatibility test | Change owner | What it blocks | Migration |
|---|---|---|---|---|---|---|---|---|---|---|
| EH3D-PHOENIX-PCB | EH3D-COMMON | compile | project container, diagnostic crate | drafted | not set | not set | to be written with the first code | the owner of the contract | reading and writing any project | to be defined |
| EH3D-PHOENIX-PCB | EH3D-CORE-LIBS | compile | geometry, trajectories, file readers, Gerber and Excellon writers | not written | not set | not set | to be written with the first code | the owner of the contract | geometry, import and export | to be defined |
| EH3D-PHOENIX-PCB | EH3D-SERVER | runtime (optional) | tasks, artifacts and projects API | drafted | not set | not set | to be written with the first code | the owner of the contract | remote tasks only | to be defined |
| EH3D-PHOENIX-PCB | EH3D-PHOENIX-CAD | conceptual | mechanical domain of the project | not written | not set | not set | to be written with the first code | the owner of the contract | mechanical steps | to be defined |
| EH3D-PHOENIX-PCB | EH3D-VALIDATION-LAB | conceptual | corpus, oracle and regression gates | not written | not set | not set | to be written with the first code | the owner of the contract | evidence of every gate | to be defined |
| EH3D-PHOENIX-PCB | EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE | conceptual | satellites that build on the design model; reached through their command line or server tasks, never linked at build time | not written | not set | not set | to be written with the first code | the owner of the contract | nothing here is blocked by them; they are blocked by this | to be defined |
| EH3D-PHOENIX-PCB | EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER | conceptual | later satellites that build on the model | not written | not set | not set | to be written with the first code | the owner of the contract | nothing here is blocked by them | to be defined |
| EH3D-PHOENIX-PCB | EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO | conceptual | consumers of the exported `.eh3dproj` package | in progress by their owners | not set | not set | to be written with the first code | the owner of the contract | the hand-off to manufacturing | to be defined |
| EH3D-PHOENIX-PCB | EH3D-SUITE-LAUNCHER | conceptual | the application registers as a module | not written | not set | not set | to be written with the first code | the owner of the contract | launcher integration | to be defined |
| EH3D-PHOENIX-PCB | EH3D-HYDRA-BRIDGE | conceptual | assembly, vision, state and twin links with HYDRA-UMC | not written | not set | not set | to be written with the first code | the owner of the contract | nothing here is blocked by it | to be defined |

Rules: repositories are built next to each other as sibling folders and each declares in its manifest the versions of the others that it needs, once those exist. A lower layer never depends on a higher one. The manifest field for build dependencies stays empty until code exists.
