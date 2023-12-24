# Structure

## Supported structure

This library supports the following structures.

- Repeat
- Repeat with variation
- Da Capo (D.C.)
- Dal Segno (D.S.)<br/>Current version does not support Coda.

## Detailed structure

``` mermaid
graph LR
  S(( )) --> CR[Compound Region] --> E(( ));
  S --> DS[D.S./D.C. Region] --> E;
```

D.S./D.C. Region:
``` mermaid
graph LR
  S(( )) --> R[Compound Region] --> E(( ));
```

In the D.S./D.C. region, Segno, Fine, and Coda can be used.

- Segno must be located before Fine.
- If this is D.S. region, one Segno must be located.
- If this is not D.S. Region, Segno should not be located.
- If this is not D.S. nor D.C. region, Fine should not be located.
- If this is not D.S. nor D.C. region, Coda should not be located.
- The number of Codas should be zero or two.
- If there are Codas and Fine, Codas should be before Fine.

Compound Region:
``` mermaid
graph LR
  S(( )) --> R[Region] --> E(( ));
  E --> S;
```

Region:
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
  RS[Repeat Start] --> SE[Sequence Region] --> RE[Repeat End];
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
