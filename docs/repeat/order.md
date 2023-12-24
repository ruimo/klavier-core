# Playing order

- Repeat region is played twice. [See repeat examples](examples.md#repeat-region)
- Variation region is played as the following. [See variation examples](examples.md#variation-region)
    - Play common region then play the variation 1.
    - Play common region then play the variation 2.
    - ...
    - Play common region then play the variation N.
- Da Capo region is played as the following. [See Da Capo examples](examples.md#da-capo)
    - Da Capo is essentially same as the Dal Segno as if Segno is located at the top of the tune (implicit Segno).
    - When the tune is auftakt, the following rule is applied.
        - The bar length where Da Capo is located at the end of the bar is equal to the full bar length calculated by the tune's rhythm, play as if the implicit Segno is located at the beginning of the second bar (skip playing the first bar when goes back from D.C.).
        - Otherwise, play as if the implicit Segno is located at the top of the tune. The length of the first bar plus the length of the D.C bar should be equal to the full bar length calculated by the tune's rhythm.
- Dal Segno region is played as the follwing.  [See Dal Segno examples](examples.md#dal-segno)
    - Play is the entire regions as normal until D.S. is found.
    - Go back to the Segno and play the regions.
        - If Fine is found, end playing.
        - For the repeat region, play only once but twice.
        - For the variation region, play the common region and then only the last variation.
        - If a Coda is found, jump to the second Coda.
