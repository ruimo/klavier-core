# Structure

## Supported structure

This library supports the following structures.

- Repeat
- Repeat with variation
- Da Capo (D.C.)
- Dal Segno (D.S.)

## Detailed structure

Region:
``` mermaid
graph LR
  S(( )) --> CR[Compound Region] --> E(( ));
  S --> DS[Simple Region] --> E;
```

Compound region can contain D.S./D.C. Segno, Fine, and Coda can be used with D.S./D.C.

- Segno must be located before Fine.
- If D.S. exists, one Segno must be located.
- Segno should not be used without D.S.
- If D.S. nor D.C. exists, Fine should not be located.
- If D.S. nor D.C. exists, Coda should not be located.
- The number of Codas should be zero or two.
- If there are Codas and Fine, Codas should be before Fine.

Compound Region:
``` mermaid
graph LR
  S(( )) --> R[Simple Region] --> E(( ));
  E --> S;
```

Simple Region:
``` mermaid
graph LR
  S(( )) --> SR[Sequence Region] --> E(( ));
  S(( )) --> R[Repeat Region] --> E(( ));
  S(( )) --> V[Variation Region] --> E(( ));
```

Sequence Region:
``` mermaid
graph LR
  S(( )) --> N[Bar] --> E(( ));
  E --> S;
```

Repeat Region:
``` mermaid
graph LR
  RS[Repeat Start Bar] --> SE[Sequence Region] --> RE[Repeat End Bar];
```

Variation Region:
``` mermaid
graph LR
  C[Common Region] --> V1[Variation 1] --> R2[Variation 2] -...-> Rn[Variation N];
```

- At least two variations shoud be located.

Common Region:
``` mermaid
graph LR
  S(( )) --> SR[Sequence Region] --> E(( ));
  S(( )) --> R[Repeat Region] --> E(( ));
  E --> S;
```

Variation:
``` mermaid
graph LR
  S(( )) --> SR[Sequence Region] --> E(( ));
```
